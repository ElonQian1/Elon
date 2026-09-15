export interface LinkEmbed { kind: 'x' | 'bilibili' | 'douyin'; id: string; url: string }
export interface LinkPreview { schema: number; url: string; title: string; site: string; author: string; image: string | null; embed: LinkEmbed | null; status: string }
export interface LinkOptions { owner: string; api: (path: string, init: RequestInit) => Promise<unknown>; open?: (preview: LinkPreview) => void; isCurrent?: () => boolean }
declare global {
  var ElonSocialLinks: {
    safeUrl(value: string): URL | null;
    links(text: string): LinkPreview[];
    mount(container: HTMLElement, text: string, options: LinkOptions): () => void;
  }
  var ElonSocialLinkViewer: { open(preview: LinkPreview): void; close(): void }
}
