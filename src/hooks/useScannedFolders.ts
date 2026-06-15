import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ScannedFolder } from "../App";

export function useScannedFolders() {
  const [scannedFolders, setScannedFolders] = useState<ScannedFolder[]>([]);
  const [folderAccessibility, setFolderAccessibility] = useState<Map<string, boolean>>(new Map());

  const loadScannedFolders = useCallback(async () => {
    try {
      const folders = await invoke<ScannedFolder[]>("get_scanned_folders");
      setScannedFolders(folders);
      const accessResults: any[] = await invoke("check_folders_accessibility");
      const accessMap = new Map<string, boolean>();
      for (const r of accessResults) {
        accessMap.set(r.id, r.accessible);
      }
      setFolderAccessibility(accessMap);
    } catch (e) { console.error("Failed to load scanned folders:", e); }
  }, []);

  const handleRemoveScannedFolder = useCallback(async (id: string) => {
    try {
      await invoke("remove_scanned_folder", { id });
      setScannedFolders(prev => prev.filter(f => f.id !== id));
      setFolderAccessibility(prev => { const m = new Map(prev); m.delete(id); return m; });
    } catch (e) { console.error("Failed to remove folder:", e); }
  }, []);

  const handleToggleScannedFolder = useCallback(async (id: string) => {
    try {
      const updated = await invoke<ScannedFolder>("toggle_scanned_folder", { id });
      setScannedFolders((prev) => prev.map((f) => f.id === id ? updated : f));
    } catch (e) {
      setScannedFolders((prev) => prev.map((f) => f.id === id ? { ...f, is_enabled: !f.is_enabled } : f));
      console.error("Failed to toggle folder:", e);
    }
  }, []);

  const handleToggleChildFolder = useCallback(async (parentPath: string, childName: string) => {
    try {
      await invoke<boolean>("toggle_child_override", { parentPath, childName });
    } catch (e) { console.error("Failed to toggle child folder:", e); }
  }, []);

  return {
    scannedFolders, setScannedFolders, folderAccessibility, setFolderAccessibility,
    loadScannedFolders, handleRemoveScannedFolder, handleToggleScannedFolder, handleToggleChildFolder,
  };
}
