/** UI snapshots only. Rust's notification services continue while the window is hidden. */
export function startVisiblePolling(
  task: () => Promise<void>,
  options: {
    onError: (error: unknown) => void;
    interval?: number;
    visibility?: Pick<Document, "visibilityState" | "addEventListener" | "removeEventListener">;
    schedule?: (callback: () => void, milliseconds: number) => number;
    cancel?: (timer: number) => void;
  },
): () => void {
  const visibility = options.visibility ?? document;
  const schedule = options.schedule ?? ((callback, ms) => window.setTimeout(callback, ms));
  const cancel = options.cancel ?? (timer => window.clearTimeout(timer));
  let timer: number | undefined;
  let stopped = false;
  let running = false;
  let resumePending = false;
  const visible = () => visibility.visibilityState === "visible";
  function clear() {
    if (timer !== undefined) cancel(timer);
    timer = undefined;
  }
  async function tick() {
    clear();
    if (stopped || !visible()) return;
    if (running) { resumePending = true; return; }
    running = true;
    try { await task(); }
    catch (error) { if (!stopped) options.onError(error); }
    finally {
      running = false;
      if (!stopped && visible()) {
        if (resumePending) { resumePending = false; void tick(); }
        else timer = schedule(() => void tick(), options.interval ?? 1000);
      }
    }
  }
  function changed() {
    clear();
    if (visible()) void tick();
  }
  visibility.addEventListener("visibilitychange", changed);
  void tick();
  return () => {
    stopped = true;
    clear();
    visibility.removeEventListener("visibilitychange", changed);
  };
}
