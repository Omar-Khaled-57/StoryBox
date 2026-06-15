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
  isIndexingPaused?: boolean;
  isAnalysisPaused?: boolean;
}

export default function ActivityIndicator({ activities, isIndexingPaused = false, isAnalysisPaused = false }: ActivityIndicatorProps) {
  const bothPaused = isIndexingPaused && isAnalysisPaused;
  const busy = activities.length > 0 || isIndexingPaused || isAnalysisPaused;
  const [open, setOpen] = useState(false);
  const [closing, setClosing] = useState(false);
  const btnRef = useRef<HTMLButtonElement>(null);
  const popupRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ top: 0, left: 0, arrowX: undefined as number | undefined, maxHeight: undefined as number | undefined, dir: "right" as "right" | "left" | "up" | "down" });

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
    const margin = 12;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    
    const isPortrait = vw < vh;

    if (isPortrait) {
      let left = b.left + b.width / 2 - pw / 2;
      left = Math.max(margin, Math.min(left, vw - pw - margin));
      let arrowX = b.left + b.width / 2 - left;
      const spaceAbove = b.top - gap - margin - 6;
      const spaceBelow = vh - b.bottom - gap - margin;

      if (spaceAbove >= 80) {
        setPos({ top: b.top - gap - 6, left, arrowX, maxHeight: spaceAbove, dir: "up" });
      } else if (spaceBelow >= 80) {
        setPos({ top: b.bottom + gap + 6, left, arrowX, maxHeight: spaceBelow, dir: "down" });
      } else if (spaceAbove > spaceBelow) {
        setPos({ top: b.top - gap - 6, left, arrowX, maxHeight: Math.max(80, spaceAbove), dir: "up" });
      } else {
        setPos({ top: b.bottom + gap + 6, left, arrowX, maxHeight: Math.max(80, spaceBelow), dir: "down" });
      }
    } else {
      const rightSpace = vw - b.right - gap;
      const fitsRight = rightSpace >= pw;
      const dir = fitsRight ? "right" : "left";
      const left = fitsRight ? b.right + gap : b.left - gap - pw;
      const top = b.top + b.height / 2;
      setPos({ top, left, arrowX: undefined, maxHeight: undefined, dir });
    }
  }, [open, activities.length]);

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
      <div style={{ position: "absolute", left: pos.left, top: pos.top, transform: pos.dir === "up" ? "translateY(-100%)" : pos.dir === "down" ? "none" : "translateY(-50%)", "--origin": pos.dir === "right" ? "0% 50%" : pos.dir === "left" ? "100% 50%" : pos.dir === "up" ? "50% 100%" : "50% 0%" } as React.CSSProperties}>
        <div ref={popupRef} className={`w-56 rounded-2xl bg-surface-900/95 backdrop-blur-xl border border-neon-500/20 shadow-2xl shadow-black/50 p-3 text-sm relative ${closing ? "animate-scale-out" : "animate-scale-in"}`}
          style={{ transformOrigin: "var(--origin)" } as React.CSSProperties}>
          <div className={`absolute w-2.5 h-2.5 bg-surface-900 border-neon-500/20 -rotate-45 ${
            pos.dir === "right" ? "left-[-5px] top-1/2 -translate-y-1/2 border-l border-t" :
            pos.dir === "left" ? "right-[-5px] top-1/2 -translate-y-1/2 border-r border-b" :
            pos.dir === "up" ? "bottom-[-5px] -translate-x-1/2 border-b border-l" :
            "top-[-5px] -translate-x-1/2 border-t border-r"
          }`} style={(pos.dir === "up" || pos.dir === "down") ? { left: pos.arrowX ?? "50%" } : undefined} />
          <div 
            className="w-full h-full overflow-y-auto [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]"
            style={{ maxHeight: pos.maxHeight ? pos.maxHeight - 26 : undefined }}
          >
            <p className="text-[0.65rem] font-bold uppercase tracking-widest text-surface-400 mb-2 px-1">
              {bothPaused ? "Paused" : busy ? "In Progress" : "Status"}
            </p>
            {activities.length === 0 && !isIndexingPaused && !isAnalysisPaused ? (
            <div className="flex items-center gap-2 px-1 py-1.5 text-emerald-400">
              <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><path d="M20 6L9 17l-5-5"/></svg>
              <span className="text-white/80 font-medium">All done</span>
            </div>
          ) : (
            <ul className="flex flex-col gap-3">
              {activities.map((act) => {
                const isPausedIndexing = act.id === "indexing" && isIndexingPaused;
                const isPausedAnalysis = act.id === "analysis" && isAnalysisPaused;
                const isPaused = isPausedIndexing || isPausedAnalysis;
                return (
                  <li key={act.id} className={`flex flex-col gap-1.5 px-1 py-2 rounded-xl border ${
                    isPausedIndexing ? "border-amber-500/20 bg-amber-500/5" :
                    isPausedAnalysis ? "border-emerald-500/20 bg-emerald-500/5" :
                    "border-neon-500/10 bg-neon-500/5"
                  }`}>
                    <div className="flex items-center gap-2.5">
                      {isPausedIndexing ? (
                        <svg className="w-3 h-3 text-amber-400 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
                          <line x1="8" y1="5" x2="8" y2="19"/><line x1="16" y1="5" x2="16" y2="19"/>
                        </svg>
                      ) : isPausedAnalysis ? (
                        <svg className="w-3 h-3 text-emerald-400 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
                          <line x1="8" y1="5" x2="8" y2="19"/><line x1="16" y1="5" x2="16" y2="19"/>
                        </svg>
                      ) : (
                        <span className="w-1.5 h-1.5 rounded-full bg-neon-400 animate-pulse shrink-0" />
                      )}
                      <span className="text-white/80 font-semibold text-[0.75rem] leading-snug">{act.label}</span>
                    </div>
                    {act.statusMessage && <span className="text-surface-400 text-[0.65rem] truncate px-4">{act.statusMessage}</span>}
                    {isPausedIndexing && <span className="text-amber-400 text-[0.65rem] truncate px-4 font-medium">Paused</span>}
                    {isPausedAnalysis && <span className="text-emerald-400 text-[0.65rem] truncate px-4 font-medium">Paused</span>}
                    {typeof act.progress === 'number' && (
                      <div className="px-4 pb-1">
                        <div className="w-full bg-surface-800 rounded-full h-1 overflow-hidden relative">
                          <div className="bg-neon-500 h-full transition-all duration-500 ease-out" style={{ width: `${act.progress}%` }} />
                          {isPaused && <div className="absolute inset-0 bg-surface-900/40 backdrop-blur-[1px]" />}
                        </div>
                      </div>
                    )}
                  </li>
                );
              })}
              {!activities.some(a => a.id === "indexing") && isIndexingPaused && (
                <li className="flex flex-col gap-1.5 px-1 py-2 rounded-xl border border-amber-500/20 bg-amber-500/5">
                  <div className="flex items-center gap-2.5">
                    <svg className="w-3 h-3 text-amber-400 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
                      <line x1="8" y1="5" x2="8" y2="19"/><line x1="16" y1="5" x2="16" y2="19"/>
                    </svg>
                    <span className="text-white/80 font-semibold text-[0.75rem] leading-snug">Indexing</span>
                  </div>
                  <span className="text-amber-400 text-[0.65rem] truncate px-4 font-medium">Paused</span>
                </li>
              )}
              {!activities.some(a => a.id === "analysis") && isAnalysisPaused && (
                <li className="flex flex-col gap-1.5 px-1 py-2 rounded-xl border border-emerald-500/20 bg-emerald-500/5">
                  <div className="flex items-center gap-2.5">
                    <svg className="w-3 h-3 text-emerald-400 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
                      <line x1="8" y1="5" x2="8" y2="19"/><line x1="16" y1="5" x2="16" y2="19"/>
                    </svg>
                    <span className="text-white/80 font-semibold text-[0.75rem] leading-snug">AI Processing</span>
                  </div>
                  <span className="text-emerald-400 text-[0.65rem] truncate px-4 font-medium">Paused</span>
                </li>
              )}
            </ul>
            )}
          </div>
        </div>
      </div>
    </div>
  );

  const mainIcon = bothPaused ? (
    <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
      <line x1="8" y1="5" x2="8" y2="19"/><line x1="16" y1="5" x2="16" y2="19"/>
    </svg>
  ) : activities.length > 0 ? (
    <svg className="w-5 h-5 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
      <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" opacity="0.3"/>
      <path d="M12 2v4" stroke="currentColor"/>
    </svg>
  ) : (
    <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M20 6L9 17l-5-5"/>
    </svg>
  );

  const badge = !bothPaused && isIndexingPaused && !isAnalysisPaused ? (
    <span className="absolute -top-0.5 -right-0.5 w-3 h-3 flex items-center justify-center">
      <svg className="w-2.5 h-2.5 text-amber-400" viewBox="0 0 12 12" fill="currentColor">
        <rect x="2" y="1" width="2.5" height="10" rx="0.8"/>
        <rect x="7.5" y="1" width="2.5" height="10" rx="0.8"/>
      </svg>
    </span>
  ) : !bothPaused && isAnalysisPaused && !isIndexingPaused ? (
    <span className="absolute -top-0.5 -right-0.5 w-3 h-3 flex items-center justify-center">
      <svg className="w-2.5 h-2.5 text-emerald-400" viewBox="0 0 12 12" fill="currentColor">
        <rect x="2" y="1" width="2.5" height="10" rx="0.8"/>
        <rect x="7.5" y="1" width="2.5" height="10" rx="0.8"/>
      </svg>
    </span>
  ) : null;

  return (
    <div className="relative flex sm:w-full sm:px-3 sm:mt-2 items-center justify-center">
      <button ref={btnRef} onClick={toggle}
        title={bothPaused ? "Processes paused" : isIndexingPaused ? "Indexing paused" : isAnalysisPaused ? "AI Analysis paused" : busy ? "Activity in progress" : "All done"}
        aria-label={bothPaused ? "Processes paused" : isIndexingPaused ? "Indexing paused" : isAnalysisPaused ? "AI Analysis paused" : busy ? "Activity in progress" : "All done"}
        className={`relative flex items-center justify-center w-10 h-10 rounded-xl border transition-all duration-300 ${
          bothPaused
            ? "border-blue-500/60 bg-blue-500/10 text-blue-400"
            : busy
              ? "border-neon-500/60 bg-neon-500/10 text-neon-400 neon-glow"
              : "border-neon-500/10 bg-surface-800/60 text-emerald-400 hover:bg-neon-500/5"
        }`}>
        {mainIcon}
        {badge}
      </button>

      {createPortal(popup, document.body)}
    </div>
  );
}
