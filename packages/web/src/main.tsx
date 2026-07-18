// SPDX-License-Identifier: MIT OR Apache-2.0

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { App } from "./App";
import { HelmProvider } from "./lib/store";
import { ChartView } from "./routes/ChartView";
import { AisTargets } from "./routes/AisTargets";
import { RoutePlanner } from "./routes/RoutePlanner";
import { SpoofingAlerts } from "./routes/SpoofingAlerts";
import "./styles.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Root element #root not found");
}

createRoot(root).render(
  <StrictMode>
    <HelmProvider>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<App />}>
            <Route index element={<ChartView />} />
            <Route path="ais" element={<AisTargets />} />
            <Route path="route" element={<RoutePlanner />} />
            <Route path="spoofing" element={<SpoofingAlerts />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </HelmProvider>
  </StrictMode>,
);
