export function openScanDialog(options: { workerFactory: () => Worker; onFriend: (id: string) => void; imageUrl?: string; onClose?: () => void }): { close(): void };
