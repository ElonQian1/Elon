export interface LinkEmbed { kind: 'x' | 'bilibili' | 'douyin'; id: string; url: string }
export interface LinkPreview { schema: number; url: string; title: string; site: string; author: string; author_avatar_data_url?: string | null; description: string; image: string | null; embed: LinkEmbed | null; status: string; source: 'server' | 'member' }
export interface LinkOptions { owner: string; api: (path: string, init: RequestInit) => Promise<unknown>; open?: (preview: LinkPreview) => void; openOriginal?: (preview: LinkPreview) => void; coverFailed?: (preview: LinkPreview) => void; isCurrent?: () => boolean; compact?: boolean; desktop?: boolean; channelsHandoff?: boolean }
export interface LinkOptions { onPreview?: (preview: LinkPreview) => void; enrichPreview?: (preview: LinkPreview) => void; actions?: (host: HTMLElement, get: () => LinkPreview) => () => void }
declare global {
  var ElonSocialLinks: {
    safeUrl(value: string): URL | null;
    channelsId(value: string): string | null;
    embed(value: string): LinkEmbed | null;
    links(text: string): LinkPreview[];
    compact(text: string): boolean;
    remember(owner: string, preview: LinkPreview, expires?: number): void;
    mount(container: HTMLElement, text: string, options: LinkOptions): () => void;
  }
  var ElonSocialLinkViewer: { open(preview: LinkPreview): void; close(): void }
}
