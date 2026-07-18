// SPDX-License-Identifier: MIT OR Apache-2.0

import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { helmService, type HelmService } from "./data";
import type {
  AisTarget,
  OwnShip,
  RoutePlan,
  SpoofAlert,
} from "./types";

interface HelmState {
  service: HelmService;
  ownShip: OwnShip;
  targets: AisTarget[];
  alerts: SpoofAlert[];
  /** Injects a GPS spoof scenario (demo / E2E). */
  startSpoof: () => void;
  /** Clears the spoof scenario. */
  stopSpoof: () => void;
  /** Plans a route via the service. */
  planRoute: (start: { lon: number; lat: number }, end: { lon: number; lat: number }) => Promise<RoutePlan>;
}

const HelmContext = createContext<HelmState | null>(null);

export function HelmProvider({ children }: { children: ReactNode }) {
  const [ownShip, setOwnShip] = useState<OwnShip>(() => helmService.getOwnShip());
  const [targets, setTargets] = useState<AisTarget[]>(() => helmService.getAisTargets());
  const [alerts, setAlerts] = useState<SpoofAlert[]>(() => helmService.getSpoofAlerts());
  const serviceRef = useRef(helmService);

  useEffect(() => {
    const unsubscribe = serviceRef.current.subscribe(() => {
      setOwnShip(serviceRef.current.getOwnShip());
      setTargets(serviceRef.current.getAisTargets());
      setAlerts(serviceRef.current.getSpoofAlerts());
    });
    return unsubscribe;
  }, []);

  const value = useMemo<HelmState>(
    () => ({
      service: serviceRef.current,
      ownShip,
      targets,
      alerts,
      startSpoof: () => serviceRef.current.startSpoof(),
      stopSpoof: () => serviceRef.current.stopSpoof(),
      planRoute: (start, end) => serviceRef.current.planRoute(start, end),
    }),
    [ownShip, targets, alerts],
  );

  return <HelmContext.Provider value={value}>{children}</HelmContext.Provider>;
}

export function useHelm(): HelmState {
  const ctx = useContext(HelmContext);
  if (!ctx) {
    throw new Error("useHelm must be used within a HelmProvider");
  }
  return ctx;
}
