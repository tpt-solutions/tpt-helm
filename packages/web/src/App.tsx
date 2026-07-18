// SPDX-License-Identifier: MIT OR Apache-2.0

import { NavLink, Outlet } from "react-router-dom";

const navItems = [
  { to: "/", label: "Chart", end: true },
  { to: "/ais", label: "AIS Targets" },
  { to: "/route", label: "Route Planner" },
  { to: "/spoofing", label: "Spoofing Alerts" },
];

export function App() {
  return (
    <div style={{ display: "flex", minHeight: "100vh" }}>
      <nav style={{ width: 200, borderRight: "1px solid #ccc", padding: 12 }}>
        <h1 style={{ fontSize: 18 }}>TPT Helm</h1>
        <ul style={{ listStyle: "none", padding: 0 }}>
          {navItems.map((item) => (
            <li key={item.to} style={{ margin: "8px 0" }}>
              <NavLink to={item.to} end={item.end}>
                {item.label}
              </NavLink>
            </li>
          ))}
        </ul>
      </nav>
      <main style={{ flex: 1, padding: 16 }}>
        <Outlet />
      </main>
    </div>
  );
}
