import { useState, useRef, useCallback } from "react";

export interface ActivityItem {
  id: string;
  label: string;
  progress?: number;
  statusMessage?: string;
}

export function useActivities() {
  const [activities, setActivities] = useState<ActivityItem[]>([]);
  const nextId = useRef(0);

  const addActivity = useCallback((label: string): string => {
    const id = String(++nextId.current);
    setActivities((prev) => [...prev, { id, label }]);
    return id;
  }, []);

  const removeActivity = useCallback((id: string) => {
    setActivities((prev) => prev.filter((a) => a.id !== id));
  }, []);

  return { activities, setActivities, addActivity, removeActivity };
}
