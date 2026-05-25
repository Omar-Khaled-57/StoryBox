import { useState, useRef, useEffect, useCallback } from "react";
import { createPortal } from "react-dom";

export interface ActivityItem {
  id: string;
  label: string;
  progress?: number;
  statusMessage?: string;
}

interface ActivityIndicatorProps {
  activities: ActivityItem[];
}

export default function ActivityIndicator({ activities }: ActivityIndicatorProps) {
  const busy = activities.length > 0;
  const [open, setOpen] = useState(false);
  const [closing, setClosing] = useState(false);
  const btnRef = useRef<HTMLButtonElement>(null);
  const popupRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ top: 0, left: 0, dir: "right" as "right" | "left" });

  const doClose = useCallback(() => {
    setClosing(true);
    setTimeout(() => { setClosing(false); setOpen(false); }, 200);
  }, []);

  const toggle = useCallback(() => {
    if (open) { doClose(); }
    else { setOpen(true); }
  }, [open, doClose]);

  const updatePos = useCallback(() => {
    if (!btnRef.current || !open) return;
    const b = btnRef.current.getBoundingClientRect();
    const pw = 224;
    const gap = 12;
    const rightSpace = window.innerWidth - b.right - gap;
    const fitsRight = rightSpace >= pw;
    const dir = fitsRight ? "right" : "left";
    const left = fitsRight ? b.right + gap : b.left - gap - pw;
    const top = b.top + b.height / 2;
    setPos({ top, left, dir });
  }, [open]);

  useEffect(() => {
    if (!open) return;
    updatePos();
    const handler = (e: MouseEvent) => {
      if (popupRef.current && !popupRef.current.contains(e.target as Node) && btnRef.current && !btnRef.current.contains(e.target as Node)) {
        doClose();
      }
    };
    document.addEventListener("mousedown", handler);
    window.addEventListener("resize", updatePos);
    return () => {
      document.removeEventListener("mousedown", handler);
      window.removeEventListener("resize", updatePos);
    };
  }, [open, updatePos, doClose]);

  const showPopup = open || closing;

  const popup = showPopup && (
    <div className="fixed inset-0 z-[9999] pointer-events-none">
      <div style={{ position: "absolute", left: pos.left, top: pos.top, transform: "translateY(-50%)", "--origin": pos.dir === "right" ? "0% 50%" : "100% 50%" } as React.CSSProperties}>
        <div ref={popupRef} className={`w-56 rounded-2xl bg-surface-900/95 backdrop-blur-xl border border-neon-500/20 shadow-2xl shadow-black/50 p-3 text-sm relative ${closing ? "animate-scale-out" : "animate-scale-in"}`}
          style={{ transformOrigin: "var(--origin)" } as React.CSSProperties}>
          <div className={`absolute w-2.5 h-2.5 bg-surface-900 border-neon-500/20 -rotate-45 top-1/2 -translate-y-1/2 ${
            pos.dir === "right" ? "left-[-5px] border-l border-t" : "right-[-5px] border-r border-b"
          }`} />
          <p className="text-[0.65rem] font-bold uppercase tracking-widest text-surface-400 mb-2 px-1">
            {busy ? "In Progress" : "Status"}
          </p>
          {activities.length === 0 ? (
            <div className="flex items-center gap-2 px-1 py-1.5 text-emerald-400">
              <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><path d="M20 6L9 17l-5-5"/></svg>
              <span className="text-white/80 font-medium">All done</span>
            </div>
          ) : (
            <ul className="flex flex-col gap-3">
              {activities.map((act) => (
                <li key={act.id} className="flex flex-col gap-1.5 px-1 py-2 rounded-xl border border-neon-500/10 bg-neon-500/5">
                  <div className="flex items-center gap-2.5">
                    <span className="w-1.5 h-1.5 rounded-full bg-neon-400 animate-pulse shrink-0" />
                    <span className="text-white/80 font-semibold text-[0.75rem] leading-snug">{act.label}</span>
                  </div>
                  {act.statusMessage && <span className="text-surface-400 text-[0.65rem] truncate px-4">{act.statusMessage}</span>}
                  {typeof act.progress === 'number' && (
                    <div className="px-4 pb-1">
                      <div className="w-full bg-surface-800 rounded-full h-1 overflow-hidden">
                        <div className="bg-neon-500 h-full transition-all duration-500 ease-out" style={{ width: `${act.progress}%` }} />
                      </div>
                    </div>
                  )}
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );

  return (
    <div className="relative flex sm:w-full sm:px-3 sm:mt-2 items-center justify-center">
      <button ref={btnRef} onClick={toggle}
        title={busy ? "Activity in progress" : "All done"}
        aria-label={busy ? "Activity in progress" : "All done"}
        className={`flex items-center justify-center w-10 h-10 rounded-xl border transition-all duration-300 ${
          busy
            ? "border-neon-500/60 bg-neon-500/10 text-neon-400 neon-glow"
            : "border-neon-500/10 bg-surface-800/60 text-emerald-400 hover:bg-neon-500/5"
        }`}>
        {busy ? (
          <svg className="w-5 h-5 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" opacity="0.3"/>
            <path d="M12 2v4" stroke="currentColor"/>
          </svg>
        ) : (
          <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M20 6L9 17l-5-5"/>
          </svg>
        )}
      </button>

      {createPortal(popup, document.body)}
    </div>
  );
}
