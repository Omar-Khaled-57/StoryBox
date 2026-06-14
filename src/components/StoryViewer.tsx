import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Story } from "../App";

interface StoryViewerProps {
  story: Story;
  onClose: () => void;
}

const SEGMENT_DURATION = 5000;

export default function StoryViewer({ story, onClose }: StoryViewerProps) {
  const [index, setIndex] = useState(0);
  const [progress, setProgress] = useState(0);
  const [paused, setPaused] = useState(false);
  const [imgSrc, setImgSrc] = useState<string | null>(null);
  const [imgLoaded, setImgLoaded] = useState(false);
  const [isFullView, setIsFullView] = useState(false);
  const [uiHidden, setUiHidden] = useState(false);
  const holdTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const heldRef = useRef(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const startTimeRef = useRef<number>(Date.now());
  const elapsedRef = useRef<number>(0);

  const currentImage = story.images[index];

  useEffect(() => {
    if (!currentImage) return;
    setImgSrc(null);
    setImgLoaded(false);
    invoke<string>("get_cached_image_base64", { id: currentImage.id, imageType: "display" })
      .then(setImgSrc)
      .catch(console.error);
  }, [currentImage]);

  const goNext = useCallback(() => {
    setIndex((i) => {
      if (i + 1 >= story.images.length) { onClose(); return i; }
      return i + 1;
    });
    setProgress(0);
    elapsedRef.current = 0;
    startTimeRef.current = Date.now();
  }, [story.images.length, onClose]);

  const goPrev = useCallback(() => {
    setIndex((i) => Math.max(0, i - 1));
    setProgress(0);
    elapsedRef.current = 0;
    startTimeRef.current = Date.now();
  }, []);

  useEffect(() => {
    if (paused || !imgLoaded) return;
    startTimeRef.current = Date.now() - elapsedRef.current;
    intervalRef.current = setInterval(() => {
      const elapsed = Date.now() - startTimeRef.current;
      elapsedRef.current = elapsed;
      setProgress(Math.min((elapsed / SEGMENT_DURATION) * 100, 100));
      if (elapsed >= SEGMENT_DURATION) goNext();
    }, 50);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, [index, paused, imgLoaded, goNext]);

  useEffect(() => { setProgress(0); elapsedRef.current = 0; }, [index]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "ArrowRight") goNext();
      if (e.key === "ArrowLeft") goPrev();
      if (e.key === "Escape") { if (isFullView) setIsFullView(false); else onClose(); }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [goNext, goPrev, onClose, isFullView]);

  const enterFullView = useCallback(() => {
    setIsFullView(true);
    setUiHidden(true);
  }, []);

  const exitFullView = useCallback(() => {
    setIsFullView(false);
    setUiHidden(false);
  }, []);

  const handlePointerDown = useCallback(() => {
    setPaused(true);
    heldRef.current = false;
    holdTimer.current = setTimeout(() => { setUiHidden(true); heldRef.current = true; }, 600);
  }, []);

  const handlePointerUp = useCallback(() => {
    if (holdTimer.current) clearTimeout(holdTimer.current);
    setUiHidden(false);
    setPaused(false);
  }, []);

  const handleClick = useCallback((fn: () => void) => (e: React.MouseEvent) => {
    if (heldRef.current) { e.preventDefault(); return; }
    fn();
  }, []);

  const subjectLabel = currentImage?.tags && currentImage.tags.length > 0
    ? currentImage.tags.join(" \u00b7 ")
    : null;

  return (
    <div className="fixed inset-0 z-[1000] flex items-center justify-center bg-surface-950/98 backdrop-blur-md animate-fade-in safe-area-content" onContextMenu={(e) => e.preventDefault()}>
      <div className="relative w-full h-full max-w-md overflow-hidden bg-surface-950 flex flex-col shadow-2xl sm:rounded-3xl sm:h-[92vh] sm:border sm:border-neon-500/20">
        {/* Cyber scanline overlay */}
        <div className="cyber-scanline z-[3] pointer-events-none transition-opacity duration-500" style={{ opacity: uiHidden ? 0 : 1 }} />

        {imgSrc && (
          <img key={`bg-${currentImage.id}`} src={imgSrc}
            className={`absolute inset-0 w-full h-full object-cover blur-3xl scale-110 transition-opacity duration-1000 z-[1] ${imgLoaded ? "opacity-20" : "opacity-0"}`} alt="" />
        )}

        {imgSrc && (
          <img key={currentImage.id} src={imgSrc}
            className={`absolute inset-0 w-full h-full object-cover transition-opacity duration-500 z-[2] ${imgLoaded ? "opacity-100" : "opacity-0"}`}
            alt="" onLoad={() => setImgLoaded(true)} onError={() => setImgLoaded(true)} />
        )}

        {!imgLoaded && (
          <div className="absolute inset-0 bg-surface-900 animate-pulse">
            <div className="absolute inset-0 bg-gradient-to-r from-transparent via-neon-500/5 to-transparent animate-shimmer" />
            <div className="absolute inset-0 flex flex-col items-center justify-center gap-3">
              <svg className="w-8 h-8 text-neon-400 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                <path opacity="0.25" d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
                <path d="M12 2v4"/>
              </svg>
              <span className="text-xs text-surface-500 font-medium tracking-widest uppercase neon-glow-text">Loading...</span>
            </div>
          </div>
        )}

        <div className="absolute inset-0 bg-gradient-to-b from-black/60 via-transparent to-black/90 z-[1] pointer-events-none transition-opacity duration-500" style={{ opacity: uiHidden ? 0 : 1 }} />

        {/* Progress bars (neon) */}
        <div className="absolute top-0 left-0 right-0 flex gap-1 px-3 pt-4 z-10 transition-opacity duration-500" style={{ opacity: uiHidden ? 0 : 1 }}>
          {story.images.map((_, i) => (
            <div key={i} className="flex-1 h-0.5 bg-white/20 rounded-full overflow-hidden">
              <div className="h-full bg-neon-400 rounded-full transition-[width] duration-[50ms] ease-linear"
                style={{ width: i < index ? "100%" : i === index ? `${progress}%` : "0%" }} />
            </div>
          ))}
        </div>

        {/* Header */}
        <div className="absolute top-8 left-0 right-0 flex items-start justify-between px-4 z-10 transition-opacity duration-500" style={{ opacity: uiHidden ? 0 : 1 }}>
          <div className="flex flex-col gap-0.5 pr-4 bg-surface-900/40 backdrop-blur-md border border-neon-500/10 rounded-xl px-3 py-2 shadow-lg shadow-black/40">
            <span className="text-sm font-bold text-white leading-tight [text-shadow:_0_2px_8px_rgb(0_0_0_/_80%)]">{story.caption}</span>
            <span className="text-[0.65rem] text-neon-300 font-bold uppercase tracking-widest neon-glow-text">
              {story.theme_type === "random" ? "Template" : "AI Story"}
            </span>
          </div>
          <button className="shrink-0 bg-[#07182a] border border-[#1684d4]/20 text-[#1684d4] w-8 h-8 rounded-xl cursor-pointer text-sm flex items-center justify-center transition-all hover:bg-[#0a2a4a] active:scale-95"
            onClick={onClose} aria-label="Close">✕</button>
        </div>

        {/* Tap zones */}
        <div className="absolute inset-0 flex z-[5] select-none">
          <div className="flex-[0.4] cursor-pointer" onClick={handleClick(goPrev)}
            role="button" tabIndex={0} aria-label="Previous image"
            onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); goPrev(); } }}
            onPointerDown={handlePointerDown} onPointerUp={handlePointerUp} onPointerLeave={handlePointerUp} />
          <div className="flex-[0.6] cursor-pointer" onClick={handleClick(goNext)}
            role="button" tabIndex={0} aria-label="Next image"
            onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); goNext(); } }}
            onPointerDown={handlePointerDown} onPointerUp={handlePointerUp} onPointerLeave={handlePointerUp} />
        </div>

        {/* Footer */}
        <div className="absolute bottom-6 left-0 right-0 flex flex-col items-start gap-2 px-5 z-10 pointer-events-none transition-opacity duration-500" style={{ opacity: uiHidden ? 0 : 1 }}>
          {subjectLabel && (
            <div className="flex flex-wrap gap-1">
              {currentImage.vibe && (
                <span className="text-[0.65rem] font-extrabold uppercase tracking-widest bg-neon-500/20 backdrop-blur-md border border-neon-500/30 text-neon-300 px-3 py-1 rounded-lg neon-glow shadow-lg shadow-black/40">
                  {currentImage.vibe}
                </span>
              )}
              {currentImage.tags!.slice(0, 3).map((tag) => (
                <span key={tag} className="text-[0.65rem] font-bold bg-surface-900/60 backdrop-blur-md text-surface-200 px-2 py-1 rounded-lg border border-neon-500/10 shadow-lg shadow-black/30">
                  {tag}
                </span>
              ))}
            </div>
          )}
          <div className="flex justify-between items-center w-full bg-surface-900/50 backdrop-blur-md border border-neon-500/10 rounded-xl px-3 py-2 shadow-lg shadow-black/40">
            <span className="text-surface-200 text-[0.7rem] font-medium tracking-wide [text-shadow:_0_1px_4px_rgb(0_0_0)]">{index + 1} / {story.images.length}</span>
            {currentImage?.path && (
              <span className="max-w-[70%] overflow-hidden text-ellipsis whitespace-nowrap text-right text-surface-200 text-[0.7rem] font-medium tracking-wide [text-shadow:_0_1px_4px_rgb(0_0_0)]">
                {currentImage.path.split(/[\\\/]/).pop()}
              </span>
            )}
          </div>
        </div>
      </div>

      {/* Full Image Toggle */}
      <button className={`fixed bottom-24 right-6 z-[1100] w-12 h-12 rounded-xl bg-[#07182a] border border-[#1684d4]/20 text-[#1684d4] flex items-center justify-center transition-all duration-500 hover:bg-[#0a2a4a] hover:scale-110 active:scale-95 shadow-2xl group ${uiHidden ? "opacity-0 pointer-events-none" : ""}`}
        onClick={(e) => { e.stopPropagation(); if (isFullView) exitFullView(); else enterFullView(); }}
        title={isFullView ? "Exit Full View" : "Full View"}
        aria-label={isFullView ? "Exit Full View" : "Full View"}>
        {isFullView ? (
          <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M4 14h6v6M20 10h-6V4M14 10l7-7M10 14l-7 7"/></svg>
        ) : (
          <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/></svg>
        )}
      </button>

      {/* Full-view overlay for landscape */}
      {isFullView && imgSrc && (
        <div className="fixed inset-0 z-[1200] bg-black/95 flex items-center justify-center animate-fade-in"
          onClick={(e) => { if (e.target === e.currentTarget) exitFullView(); }}>
          <img src={imgSrc}
            className="max-w-[95vw] max-h-[95vh] w-auto h-auto object-contain rounded-2xl shadow-2xl animate-zoom-in"
            alt="" style={{ aspectRatio: "auto" }} />
        </div>
      )}
    </div>
  );
}
