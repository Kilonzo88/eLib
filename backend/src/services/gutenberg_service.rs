use lol_html::{element, text, HtmlRewriter, MemorySettings, Settings};
use reqwest::{header, Client};
use std::time::Duration;

/// The 32 books exposed in your MVP.
/// Tuple of (gutenberg_id, canonical_title, author)
pub const GUTENBERG_CATALOGUE: &[(u32, &str, &str)] = &[
    (84,   "Frankenstein",                          "Mary Shelley"),
    (1342, "Pride and Prejudice",                   "Jane Austen"),
    (2701, "Moby-Dick",                             "Herman Melville"),
    (11,   "Alice's Adventures in Wonderland",      "Lewis Carroll"),
    (1661, "The Adventures of Sherlock Holmes",     "Arthur Conan Doyle"),
    (98,   "A Tale of Two Cities",                  "Charles Dickens"),
    (1260, "Jane Eyre",                             "Charlotte Brontë"),
    (174,  "The Picture of Dorian Gray",            "Oscar Wilde"),
    (345,  "Dracula",                               "Bram Stoker"),
    (76,   "Adventures of Huckleberry Finn",        "Mark Twain"),
    (1080, "A Modest Proposal",                     "Jonathan Swift"),
    (2591, "Grimms' Fairy Tales",                   "Brothers Grimm"),
    (4300, "Ulysses",                               "James Joyce"),
    (5200, "Metamorphosis",                         "Franz Kafka"),
    (1232, "The Prince",                            "Niccolò Machiavelli"),
    (2554, "Crime and Punishment",                  "Fyodor Dostoevsky"),
    (2600, "War and Peace",                         "Leo Tolstoy"),
    (768,  "Wuthering Heights",                     "Emily Brontë"),
    (514,  "Little Women",                          "Louisa May Alcott"),
    (1184, "The Count of Monte Cristo",             "Alexandre Dumas"),
    (844,  "The Importance of Being Earnest",       "Oscar Wilde"),
    (16,   "Peter Pan",                             "J. M. Barrie"),
    (1400, "Great Expectations",                    "Charles Dickens"),
    (46,   "A Christmas Carol",                     "Charles Dickens"),
    (730,  "Oliver Twist",                          "Charles Dickens"),
    (120,  "Treasure Island",                       "Robert Louis Stevenson"),
    (161,  "Sense and Sensibility",                 "Jane Austen"),
    (1952, "The Yellow Wallpaper",                  "Charlotte Perkins Gilman"),
    (219,  "Heart of Darkness",                     "Joseph Conrad"),
    (23,   "Narrative of the Life of Frederick Douglass", "Frederick Douglass"),
    (100,  "The Complete Works of Shakespeare",     "William Shakespeare"),
    (3207, "Leviathan",                             "Thomas Hobbes"),
];

pub fn build_gutenberg_client() -> Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        // Brave on Ubuntu — accurate to your environment
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
            .parse()
            .unwrap(),
    );
    headers.insert(header::ACCEPT_LANGUAGE, "en-GB,en;q=0.9".parse().unwrap());
    headers.insert(
        header::ACCEPT,
        "application/epub+zip,*/*;q=0.8".parse().unwrap(),
    );

    Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(120)) // EPUBs can be large
        .connect_timeout(Duration::from_secs(15))
        .gzip(true) //Compression  from 1992
        .brotli(true) //Most recent compression engine(Google 2015)
        .build()
        .expect("Failed to build Gutenberg HTTP client")
}

/// Tries each URL candidate in order, returning the raw EPUB bytes on first success.
pub async fn fetch_epub(book_id: u32, client: &Client) -> anyhow::Result<bytes::Bytes> {
    let candidates = [
        format!("https://www.gutenberg.org/ebooks/{book_id}.epub.noimages"),
        format!("https://www.gutenberg.org/ebooks/{book_id}.epub.images"),
        format!("https://gutenberg.pglaf.org/cache/epub/{book_id}/pg{book_id}.epub"),
    ];

    for (attempt, url) in candidates.iter().enumerate() {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_secs(attempt as u64)).await;
        }

        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let bytes = resp.bytes().await?;
                println!("    Fetched from: {url}");
                return Ok(bytes);
            }
            Ok(resp) => {
                println!("    HTTP {} from {url} — trying next mirror", resp.status());
            }
            Err(e) => {
                println!("    Network error from {url}: {e} — trying next mirror");
            }
        }
    }

    anyhow::bail!("All mirrors exhausted for book {book_id}")
}

/// Extracts raw HTML strings from each chapter inside an EPUB.
/// Returns a Vec of HTML strings, one per spine item (chapter).
pub fn extract_chapters_from_epub(epub_bytes: &[u8]) -> anyhow::Result<Vec<String>> {
    use epub::doc::EpubDoc;
    use std::io::Cursor;

    let cursor = Cursor::new(epub_bytes);
    let mut doc = EpubDoc::from_reader(cursor)
        .map_err(|e| anyhow::anyhow!("EPUB parse error: {e}"))?;

    let mut chapters: Vec<String> = Vec::new();

    // Iterate through the spine — the ordered reading sequence
    let spine_len = doc.spine.len();
    for i in 0..spine_len {
        // go_to_path requires the spine item id
        let id = doc.spine[i].idref.clone();
        if let Some((content, _mime)) = doc.get_resource(&id) {
            if let Ok(html) = String::from_utf8(content) {
                // Skip nav/toc chapters — they're not reading content
                if html.contains("epub:type=\"toc\"") || html.contains("epub:type=\"nav\"") {
                    continue;
                }
                chapters.push(html);
            }
        }
    }

    Ok(chapters)
}

/// Chunks a chapter's HTML into segments of approximately `target_words` words,
/// always sealing a chunk at the end of a complete <p>, <li>, or heading element.
/// A sentence is never split mid-way.
pub fn chunk_chapter(html: &str, target_words: usize) -> Vec<String> {
    use std::rc::Rc;
    use std::cell::RefCell;

    let segments = Rc::new(RefCell::new(Vec::new()));
    let current_chunk = Rc::new(RefCell::new(String::new()));
    let current_words = Rc::new(RefCell::new(0usize));
    let current_para = Rc::new(RefCell::new(String::new()));

    let seg_1 = segments.clone();
    let chk_1 = current_chunk.clone();
    let wrd_1 = current_words.clone();
    let par_1 = current_para.clone();

    let par_2 = current_para.clone();

    fn seal_para(
        para: &mut String,
        chunk: &mut String,
        words: &mut usize,
        segs: &mut Vec<String>,
        target: usize,
    ) {
        let trimmed = para.trim().to_string();
        para.clear();

        if trimmed.is_empty() {
            return;
        }

        let para_words = trimmed.split_whitespace().count();

        if *words + para_words > target && *words > 0 {
            segs.push(chunk.trim().to_string());
            chunk.clear();
            *words = 0;
        }

        chunk.push_str(&trimmed);
        chunk.push('\n');
        *words += para_words;
    }

    let result = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![
                element!("p, li, blockquote, h1, h2, h3, h4, h5, dd, dt", move |el| {
                    let seg_3 = seg_1.clone();
                    let chk_3 = chk_1.clone();
                    let wrd_3 = wrd_1.clone();
                    let par_3 = par_1.clone();
                    el.on_end_tag(Box::new(move |_end: &mut lol_html::html_content::EndTag| {
                        seal_para(
                            &mut par_3.borrow_mut(),
                            &mut chk_3.borrow_mut(),
                            &mut wrd_3.borrow_mut(),
                            &mut seg_3.borrow_mut(),
                            target_words,
                        );
                        Ok(())
                    }) as _)?;
                    Ok(())
                }),
                text!("p, li, blockquote, h1, h2, h3, h4, h5, dd, dt", move |t| {
                    par_2.borrow_mut().push_str(t.as_str());
                    Ok(())
                }),
            ],
            memory_settings: MemorySettings {
                max_allowed_memory_usage: 2 * 1024 * 1024,
                ..MemorySettings::default()
            },
            ..Settings::default()
        },
        |_: &[u8]| {},
    )
    .write(html.as_bytes());

    if let Err(e) = result {
        println!("    Warning: lol-html write error (partial chunk kept): {e}");
    }

    let final_para = current_para.borrow_mut();
    let mut final_chunk = current_chunk.borrow_mut();
    let mut final_segs = segments.borrow_mut();

    if !final_para.trim().is_empty() {
        final_chunk.push_str(final_para.trim());
        final_chunk.push('\n');
    }
    if !final_chunk.trim().is_empty() {
        final_segs.push(final_chunk.trim().to_string());
    }

    final_segs.clone()
}