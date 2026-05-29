import type { Metadata } from "next";
import { IBM_Plex_Serif, Mona_Sans } from "next/font/google";
import "./globals.css";
import Navbar from "@/components/ui/Navbar";
import ClaimTracker from "@/components/ui/ClaimTracker";
import { ClerkProvider } from "@clerk/nextjs";

const ibmPlexSerif = IBM_Plex_Serif({
  variable: "--font-ibm-plex-serif",
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
  display: "swap",
});

const monaSans = Mona_Sans({
  variable: "--font-mona-sans",
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
  display: "swap",
});

export const metadata: Metadata = {
  title: "eLib",
  description: "Transform your books into interactive AI conversations. Upload PDFs, and chat with an AI agent about the contents using voice",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={`${ibmPlexSerif.variable} ${monaSans.variable} relative font-sans antialiased`}>
        <ClerkProvider>
          <Navbar />
          <ClaimTracker />
          <main className="max-w-6xl mx-auto px-4 md:px-8 pt-16 md:pt-24 pb-0 md:pb-12">
            {children}
          </main>
        </ClerkProvider>
      </body>
    </html>
  );
}
