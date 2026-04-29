import UploadForm from '@/components/ui/UploadForm'

export default function Page() {
  return (
    <div className="mx-auto max-w-3xl space-y-10">
      <section className="flex flex-col gap-5">
        <h1 className="text-3xl md:text-4xl font-serif font-bold mb-4 text-[var(--primary)]">
          Add a New Book
        </h1>
        <p className="text-[var(--muted-foreground)]">
          Upload a PDF to generate your interactive interview
        </p>
      </section>

      <UploadForm />
    </div>
  )
}