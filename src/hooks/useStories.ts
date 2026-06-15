import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Story } from "../App";

export function useStories() {
  const [stories, setStories] = useState<Story[]>([]);

  const loadStories = useCallback(async () => {
    try {
      const result = await invoke<Story[]>("get_stories");
      setStories(result);
    } catch (e) { console.error("Failed to load stories:", e); }
  }, []);

  const handleDeleteStory = useCallback(async (id: string) => {
    try {
      await invoke("delete_story", { id });
      setStories((prev) => prev.filter((s) => s.id !== id));
    } catch (e) { console.error(e); }
  }, []);

  const handleTogglePin = useCallback(async (id: string) => {
    try {
      const is_pinned = await invoke<boolean>("toggle_story_pin", { id });
      setStories((prev) => [...prev].map((s) => (s.id === id ? { ...s, is_pinned } : s))
        .sort((a, b) => { if (a.is_pinned !== b.is_pinned) return a.is_pinned ? -1 : 1; return (new Date(b.created_at).getTime() || 0) - (new Date(a.created_at).getTime() || 0); }));
    } catch (e) { console.error(e); }
  }, []);

  const handleToggleFavorite = useCallback(async (id: string) => {
    try {
      const is_favorite = await invoke<boolean>("toggle_story_favorite", { id });
      setStories((prev) => prev.map((s) => (s.id === id ? { ...s, is_favorite } : s)));
    } catch (e) { console.error(e); }
  }, []);

  return { stories, setStories, loadStories, handleDeleteStory, handleTogglePin, handleToggleFavorite };
}
