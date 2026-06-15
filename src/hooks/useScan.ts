import { useState } from "react";

export function useScan() {
  const [scanLog, setScanLog] = useState("Ready to index");
  const [isScanning, setIsScanning] = useState(false);
  const [showNoPhotosPopup, setShowNoPhotosPopup] = useState(false);

  return { scanLog, setScanLog, isScanning, setIsScanning, showNoPhotosPopup, setShowNoPhotosPopup };
}
