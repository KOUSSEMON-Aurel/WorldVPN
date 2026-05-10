import { useEffect, useRef } from "react";

// Inform TS that global `L` exists since it is injected via CDN
declare const L: any;

const COUNTRY_COORDS: Record<string, [number, number]> = {
  'US': [37.0902, -95.7129],
  'GB': [55.3781, -3.4360],
  'FR': [46.2276, 2.2137],
  'DE': [51.1657, 10.4515],
  'JP': [36.2048, 138.2529],
  'KR': [35.9078, 127.7669],
  'SG': [1.3521, 103.8198],
  'HK': [22.3193, 114.1694],
  'NL': [52.1326, 5.2913],
  'CA': [56.1304, -106.3468],
  'AU': [-25.2744, 133.7751],
  'RU': [61.5240, 105.3188],
  'IN': [20.5937, 78.9629],
  'BR': [-14.2350, -51.9253],
  'IT': [41.8719, 12.5674],
  'ES': [40.4637, -3.7492],
  'SE': [60.1282, 18.6435],
  'CH': [46.8182, 8.2275],
};

export function LeafletMap({ nodes, onConnect, nodeGroup }: any) {
  const mapRef = useRef<any>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (typeof L === "undefined") {
      console.error("Leaflet (L) is not loaded!");
      return;
    }
    if (!containerRef.current) return;
    if (mapRef.current) return;

    try {
      const map = L.map(containerRef.current, {
        center: [20, 0],
        zoom: 2,
        zoomControl: false,
      });
      mapRef.current = map;

      L.tileLayer(
        "https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png",
        {
          attribution: '&copy; OpenStreetMap &copy; CARTO',
          subdomains: "abcd",
          maxZoom: 20,
        }
      ).addTo(map);

      L.control.zoom({ position: "bottomright" }).addTo(map);
    } catch (e) {
      console.error("Map initialization failed", e);
    }

    return () => {
      if (mapRef.current) {
        mapRef.current.remove();
        mapRef.current = null;
      }
    };
  }, []);

  useEffect(() => {
    const map = mapRef.current;
    if (!map || typeof L === "undefined") return;

    const countryGroups: Record<string, any[]> = {};
    nodes.forEach((n: any) => {
      const code = n.country_code || "US";
      if (!countryGroups[code]) countryGroups[code] = [];
      countryGroups[code].push(n);
    });

    if (!map._customMarkers) map._customMarkers = [];
    map._customMarkers.forEach((m: any) => m.remove());
    map._customMarkers = [];

    const isCommunity = nodeGroup === "COMMUNITY";
    const baseColor = isCommunity ? "#00f2ea" : "#ff00ea";

    Object.keys(countryGroups).forEach((cc) => {
      const countryNodes = countryGroups[cc];
      countryNodes.sort((a: any, b: any) => (b.bandwidth_mbps || 0) - (a.bandwidth_mbps || 0));
      const count = countryNodes.length;

      const first = countryNodes[0];
      // Use node coords or fallback to country mapping
      let lat = first.lat || 0;
      let lon = first.lon || 0;

      if (lat === 0 && lon === 0) {
        const coords = COUNTRY_COORDS[cc] || [0, 0];
        lat = coords[0];
        lon = coords[1];
      }

      const badgeHtml = count > 1 ? `<div class="custom-marker-badge">&times;${count}</div>` : '';
      const icon = L.divIcon({
        className: "custom-marker-wrapper",
        html: `<div class="custom-marker-container" style="color: ${baseColor};">
          <div class="custom-marker-dot"></div>
          ${badgeHtml}
          </div>`,
        iconSize: [24, 24],
        iconAnchor: [12, 12],
      });

      const marker = L.marker([lat, lon], { icon }).addTo(map);

      // Calculate protocol breakdown
      const protocolCounts: Record<string, number> = {};
      countryNodes.forEach(n => {
        const p = n.protocol || "Unknown";
        protocolCounts[p] = (protocolCounts[p] || 0) + 1;
      });
      const protocolBadges = Object.entries(protocolCounts).map(([proto, count]) =>
        `<span class="text-[8px] px-1 py-0.5 mt-1 rounded text-white/70" style="border: 1px solid white; display: inline-block;">${count}x ${proto}</span>`
      ).join(' ');

      const popupHtml = document.createElement("div");
      popupHtml.className = "flex flex-col min-w-[250px]";
      popupHtml.innerHTML = `
        <div class="flex justify-between items-start mb-3 border-b border-white/10 pb-2">
          <div class="flex flex-col">
             <span class="text-white font-black text-sm tracking-wider flex items-center gap-2">${cc} <span class="text-xs text-text-muted">(${count} servers)</span></span>
             <div class="flex gap-1 flex-wrap mt-1">
                ${protocolBadges}
             </div>
          </div>
          <span class="text-[8px] px-2 py-0.5 rounded-full font-bold uppercase mt-1" style="background: ${baseColor}20; color: ${baseColor}; border: 1px solid ${baseColor}20;">
            ${nodeGroup}
          </span>
        </div>
        <div class="popup-node-list max-h-[200px] overflow-y-auto space-y-2 pr-1"></div>
      `;

      const listContainer = popupHtml.querySelector(".popup-node-list")!;
      countryNodes.forEach((n: any) => {
        const row = document.createElement("div");
        row.className = "flex flex-col bg-white/5 rounded-lg p-2 border border-white/5 hover:border-white/20 transition-all";
        row.innerHTML = `
          <div class="flex justify-between items-center text-[10px] mb-1.5">
            <span class="text-text-muted/80 flex items-center gap-1"><span class="w-1.5 h-1.5 rounded-full" style="background: ${baseColor}"></span> ${n.provider || (isCommunity ? "Decentralized Peer" : "Public Node")}</span>
            <span class="font-bold text-white">${n.protocol || "???"}</span>
          </div>
          <div class="flex justify-between items-center text-[10px] mb-2">
            <div>
              <span class="text-text-muted/60">Ping:</span> <span class="text-success font-mono font-bold">${n.latency_ms || "--"}ms</span>
            </div>
            <div>
              <span class="text-text-muted/60">BW:</span> <span class="text-primary font-mono font-bold">${n.bandwidth_mbps || "--"}M</span>
            </div>
          </div>
        `;
        const btn = document.createElement("button");
        btn.className = "w-full py-1 text-[10px] font-bold rounded bg-surface border border-white/10 hover:bg-white/10 transition-colors uppercase tracking-wider text-white";
        btn.innerText = "Connect";
        btn.onclick = () => {
          onConnect(n);
          map.closePopup();
        };
        row.appendChild(btn);
        listContainer.appendChild(row);
      });

      marker.bindPopup(popupHtml, { minWidth: 250, autoPanPadding: [20, 20] });
      map._customMarkers.push(marker);
    });

  }, [nodes, nodeGroup, onConnect]);

  if (typeof L === "undefined") {
    return (
      <div className="w-full h-full flex flex-col items-center justify-center bg-surface/50 gap-4">
        <div className="w-10 h-10 border-4 border-primary border-t-transparent rounded-full animate-spin"></div>
        <p className="text-xs text-text-muted font-bold text-center uppercase tracking-widest">Warping to Matrix...</p>
      </div>
    );
  }

  return <div ref={containerRef} className="w-full h-full z-10 rounded-3xl" />;
}
