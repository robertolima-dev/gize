import { useEffect, useState } from "react";
import { Icon } from "./icons";

export function toast(message: string): void {
  window.dispatchEvent(new CustomEvent("gize-toast", { detail: message }));
}

export function ToastHost() {
  const [msg, setMsg] = useState<string | null>(null);
  useEffect(() => {
    let timer: number | undefined;
    function onToast(e: Event) {
      setMsg((e as CustomEvent<string>).detail);
      window.clearTimeout(timer);
      timer = window.setTimeout(() => setMsg(null), 2400);
    }
    window.addEventListener("gize-toast", onToast);
    return () => {
      window.removeEventListener("gize-toast", onToast);
      window.clearTimeout(timer);
    };
  }, []);
  return (
    <div className={"toast" + (msg ? " show" : "")} role="status" aria-live="polite">
      <Icon name="check" className="ok" />
      <span>{msg}</span>
    </div>
  );
}
