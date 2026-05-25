import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Story, AiHealthStatus } from "../App";

interface HomeFeedProps {
  stories: Story[];
  onStoryClick: (story: Story) => void;
  onGenerate: () => void;
  isGenerating: boolean;
  onDeleteStory: (id: string) => void;
  onTogglePin: (id: string) => void;
  onToggleFavorite: (id: string) => void;
  aiHealth: AiHealthStatus | null;
  onNavigateToSettings: (sectionId: string) => void;
  title?: string;
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "Unknown date";
  try { return new Date(dateStr).toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" }); }
  catch { return dateStr; }
}

export default function HomeFeed({
  stories, onStoryClick, onGenerate, isGenerating, onDeleteStory, onTogglePin, onToggleFavorite,
  aiHealth, onNavigateToSettings, title = "StoryBox3"
}: HomeFeedProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [startX, setStartX] = useState(0);
  const [scrollLeft, setScrollLeft] = useState(0);
  const [dragged, setDragged] = useState(false);

  const handleMouseDown = (e: React.MouseEvent) => {
    if (!scrollRef.current) return;
    setIsDragging(true); setDragged(false);
    setStartX(e.pageX - scrollRef.current.offsetLeft);
    setScrollLeft(scrollRef.current.scrollLeft);
  };
  const handleMouseUp = () => setIsDragging(false);
  const handleMouseLeave = () => setIsDragging(false);
  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging || !scrollRef.current) return;
    e.preventDefault();
    const x = e.pageX - scrollRef.current.offsetLeft;
    const walk = (x - startX) * 2;
    if (Math.abs(x - startX) > 5) setDragged(true);
    scrollRef.current.scrollLeft = scrollLeft - walk;
  };

  return (
    <div className="flex flex-col gap-10 p-10 h-full overflow-y-auto overflow-x-hidden w-full">
      <header className="flex items-end justify-between gap-4 animate-fade-in">
        <div className="flex items-center gap-4">
          <div>
            <h1 className="text-3xl sm:text-4xl font-extrabold tracking-tight leading-tight pb-1 mb-2 bg-gradient-to-br from-neon-300 to-neon-500 bg-clip-text text-transparent neon-glow-text">
              {title}
            </h1>
            <p className="text-sm text-surface-400 m-0">Your memories, augmented by AI.</p>
          </div>
        </div>
        <button
          className={`flex items-center gap-2 px-6 py-3 rounded-xl border-none text-white font-bold text-sm cursor-pointer transition-all duration-300 shadow-lg whitespace-nowrap ${
            isGenerating
              ? "bg-surface-700 opacity-80 cursor-not-allowed"
              : "btn-primary"
          }`}
          onClick={onGenerate} disabled={isGenerating}
        >
          {isGenerating ? (
            <><span className="w-3.5 h-3.5 border-2 border-white/40 border-t-white rounded-full animate-spin" /> Generating...</>
          ) : (
            <><PlusIcon /> New Story</>
          )}
        </button>
      </header>

      {aiHealth && !aiHealth.available && (
        <div className="flex items-center gap-4 p-5 bg-red-500/10 border border-red-500/20 rounded-2xl animate-in cursor-pointer hover:bg-red-500/15 transition-colors group"
          onClick={() => onNavigateToSettings("ai-health-check")}
          role="button" tabIndex={0}
          onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onNavigateToSettings("ai-health-check"); }}}>
          <div className="w-10 h-10 rounded-xl bg-red-500/20 flex items-center justify-center text-red-500 shrink-0 group-hover:scale-110 transition-transform">
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/></svg>
          </div>
          <div className="flex flex-col gap-0.5">
            <h4 className="text-sm font-bold text-red-100 flex items-center gap-2">
              AI Offline
              <span className="text-[10px] bg-red-500/20 px-1.5 py-0.5 rounded text-red-300 opacity-0 group-hover:opacity-100 transition-opacity">Settings &rarr;</span>
            </h4>
            <p className="text-xs text-red-400 font-medium">Configure your OpenRouter API key in Settings to enable AI features.</p>
          </div>
        </div>
      )}

      {stories.length === 0 ? (
        <div className="flex flex-col items-center justify-center gap-4 flex-1 text-center py-16 px-8 animate-fade-in">
          <div className="text-6xl opacity-30 grayscale">{title.includes("Favorites") ? <HeartIcon /> : <CameraIcon />}</div>
          <h2 className="text-2xl font-semibold text-surface-50">
            {title.includes("Favorites") ? "No favorites yet" : "No stories yet"}
          </h2>
          <p className="text-surface-400 max-w-xs leading-relaxed">
            {title.includes("Favorites")
              ? "Save your favorite stories to see them here."
              : 'Click "Add" in the sidebar to add a folder, then generate your first story.'}
          </p>
        </div>
      ) : (
        <section className="animate-slide-up">
          <h2 className="text-sm font-semibold text-surface-400 uppercase tracking-widest mb-5">Recent Stories</h2>
          <div ref={scrollRef}
            onMouseDown={handleMouseDown} onMouseLeave={handleMouseLeave} onMouseUp={handleMouseUp} onMouseMove={handleMouseMove}
            className={`flex py-4 gap-5 overflow-x-auto pb-6 -mx-10 px-10 scrollbar-hide ${isDragging ? 'cursor-grabbing select-none' : 'cursor-grab'}`}>
            {stories.map((story) => (
              <StoryCard key={story.id} story={story}
                onClick={() => { if (!dragged) onStoryClick(story); }}
                onDelete={() => onDeleteStory(story.id)}
                onTogglePin={() => onTogglePin(story.id)}
                onToggleFavorite={() => onToggleFavorite(story.id)} />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

function StoryCard({ story, onClick, onDelete, onTogglePin, onToggleFavorite }: {
  story: Story; onClick: () => void; onDelete: () => void; onTogglePin: () => void; onToggleFavorite: () => void;
}) {
  const firstImage = story.images[0];
  const imageCount = story.images.length;
  const [imgSrc, setImgSrc] = useState<string | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [menuClosing, setMenuClosing] = useState(false);

  const closeMenu = () => {
    if (!menuOpen) return;
    setMenuClosing(true);
    setTimeout(() => { setMenuClosing(false); setMenuOpen(false); }, 200);
  };

  useEffect(() => {
    if (firstImage) {
      invoke<string>("get_cached_image_base64", { id: firstImage.id, imageType: "thumb" }).then(setImgSrc).catch(console.error);
    }
  }, [firstImage]);

  return (
    <div className="animated-border-wrap min-w-[280px] max-w-[280px] h-[440px] shrink-0 snap-center cursor-pointer transition-all duration-500 ease-out hover:-translate-y-2 hover:shadow-neon-500/20 group"
      onClick={onClick} role="button" tabIndex={0} onKeyDown={(e) => e.key === "Enter" && onClick()}>
      <div className="animated-border-inner bg-surface-800 shadow-lg">
        <div className="absolute inset-0 transition-transform duration-700 group-hover:scale-105">
        {imgSrc ? (
          <img src={imgSrc} className="w-full h-full object-cover animate-fade-in" alt=""
            onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }} />
        ) : (
          <div className="w-full h-full bg-gradient-to-br from-surface-800 to-surface-900" />
        )}
      </div>

      <div className="absolute top-3 right-3 z-20">
        <button
          className={`w-8 h-8 rounded-xl flex items-center justify-center transition-all duration-200 border border-[#1684d4]/20 cursor-pointer ${
            menuOpen ? "bg-[#07182a] text-[#1684d4]" : "bg-[#07182a] text-[#1684d4] hover:bg-[#0a2a4a]"
          }`}
          onClick={(e) => { e.stopPropagation(); if (menuOpen) closeMenu(); else setMenuOpen(true); }}
          aria-label="Story options">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/>
          </svg>
        </button>
        {menuOpen && (
          <>
            <div className="fixed inset-0 z-10" onClick={(e) => { e.stopPropagation(); closeMenu(); }} />

            <div className={`absolute top-10 right-0 w-44 bg-surface-900/95 backdrop-blur-xl border border-neon-500/20 rounded-xl shadow-2xl overflow-hidden z-20 ${menuClosing ? "animate-scale-out" : "animate-scale-in"}`}>
              <button className="w-full text-left px-4 py-3 text-sm font-medium text-surface-200 hover:bg-neon-500/10 hover:text-neon-200 transition-colors flex items-center gap-3 border-none bg-transparent cursor-pointer"
                onClick={(e) => { e.stopPropagation(); onTogglePin(); closeMenu(); }}>
                <svg className={`w-4 h-4 ${story.is_pinned ? 'text-neon-400 fill-neon-400' : ''}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><line x1="12" y1="17" x2="12" y2="22"/><path d="M5 17h14v-1.76a2 2 0 00-1.11-1.79l-1.78-.9A2 2 0 0115 10.76V6a3 3 0 00-3-3 3 3 0 00-3 3v4.76a2 2 0 01-1.11 1.79l-1.78.9A2 2 0 005 15.24V17z"/></svg>
                {story.is_pinned ? 'Unpin' : 'Pin to Top'}
              </button>
              <button className="w-full text-left px-4 py-3 text-sm font-medium text-surface-200 hover:bg-neon-500/10 hover:text-neon-200 transition-colors flex items-center gap-3 border-none bg-transparent cursor-pointer"
                onClick={(e) => { e.stopPropagation(); onToggleFavorite(); closeMenu(); }}>
                <svg className={`w-4 h-4 ${story.is_favorite ? 'text-cyber-400 fill-cyber-400' : ''}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/></svg>
                {story.is_favorite ? 'Unsave' : 'Save'}
              </button>
              <div className="h-[1px] bg-neon-500/10 my-1" />
              <button className="w-full text-left px-4 py-3 text-sm font-medium text-red-400 hover:bg-red-500/10 transition-colors flex items-center gap-3 border-none bg-transparent cursor-pointer"
                onClick={(e) => { e.stopPropagation(); onDelete(); closeMenu(); }}>
                <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2M10 11v6M14 11v6"/></svg>
                Delete
              </button>
            </div>
          </>
        )}
      </div>

      <div className="absolute inset-0 bg-gradient-to-b from-black/10 via-transparent to-black/90" />

      {story.is_pinned && (
        <div className="absolute top-3 left-3 bg-neon-500/20 backdrop-blur-md border border-neon-500/30 text-neon-300 rounded-xl px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider flex items-center gap-1 shadow-lg z-10">
          <svg className="w-3 h-3 fill-current rotate-45" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><line x1="12" y1="17" x2="12" y2="22"/><path d="M5 17h14v-1.76a2 2 0 00-1.11-1.79l-1.78-.9A2 2 0 0115 10.76V6a3 3 0 00-3-3 3 3 0 00-3 3v4.76a2 2 0 01-1.11 1.79l-1.78.9A2 2 0 005 15.24V17z"/></svg>
          Pinned
        </div>
      )}

      <div className="absolute inset-0 flex flex-col justify-end p-5 gap-2">
        <div className="flex gap-1 mb-2">
          {story.images.slice(0, 8).map((_, i) => (
            <div key={i} className="flex-1 h-1 bg-white/20 rounded-full" />
          ))}
        </div>
        <div className="flex items-center justify-between mb-1">
          <span className="text-xs font-bold bg-neon-500/20 backdrop-blur-md border border-neon-500/20 px-2.5 py-1 rounded-lg text-neon-300">
            {story.theme_type === "random" ? "Template" : "AI"}
          </span>
          <span className="text-xs text-surface-200 bg-surface-900/50 backdrop-blur-sm border border-neon-500/10 px-2 py-1 rounded-lg [text-shadow:_0_1px_4px_rgb(0_0_0)]">{formatDate(story.created_at)}</span>
        </div>
        <h3 className="text-lg font-bold text-white leading-tight [text-shadow:_0_2px_12px_rgb(0_0_0_/_85%),_0_0_30px_rgb(0_0_0_/_50%)]">{story.caption}</h3>
        <small className="w-fit text-xs text-surface-200 bg-surface-900/50 backdrop-blur-sm border border-neon-500/10 px-2 py-1 rounded-lg [text-shadow:_0_1px_4px_rgb(0_0_0)]">{imageCount} photo{imageCount !== 1 ? "s" : ""}</small>
      </div>
    </div>
    </div>
  );
}

function PlusIcon() {
  return <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>;
}
function CameraIcon() {
  return <svg className="w-16 h-16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1"><path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"/><circle cx="12" cy="13" r="4"/></svg>;
}
function HeartIcon() {
  return <svg className="w-16 h-16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1"><path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"/></svg>;
}
