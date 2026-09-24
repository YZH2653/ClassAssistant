interface ErrorBannerProps {
  error: string | null;
  onDismiss: () => void;
}

export function ErrorBanner({ error, onDismiss }: ErrorBannerProps) {
  if (!error) return null;
  return (
    <div className="flex items-center justify-between rounded-lg border border-rose-200 bg-rose-50 px-4 py-2 text-sm text-rose-700">
      <span>{error}</span>
      <button className="ml-4 text-rose-500 hover:text-rose-700" onClick={onDismiss}>
        关闭
      </button>
    </div>
  );
}
