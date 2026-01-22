let map;
let socket;

const locationMarkers = new Map(); // key -> { marker, activeIds: Set, lastLatLng }
const userToLocationKey = new Map(); // userId -> locationKey

(function injectPastStyles() {
    const css = [
        ".leaflet-popup-content .past { color: #777; }",
        ".leaflet-popup-content .current { color: #222; }",
    ].join("\n");
    const s = document.createElement("style");
    s.type = "text/css";
    s.appendChild(document.createTextNode(css));
    document.head.appendChild(s);
})();

function escapeHtml(s) {
    return String(s || "")
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
}

function popupHtml(label, activeCount) {
    const hasActive = activeCount > 0;
    const statusClass = hasActive ? "current" : "past";
    if (!hasActive) {
        return `<div class="${statusClass}"><b>${escapeHtml(label)}</b></div>`;
    }
    return `<div class="${statusClass}"><b>${escapeHtml(label)}</b><br/>Online</div>`;
}

const styleCurrent = {
    radius: 7,
    fillColor: "#4fc3f7",
    color: "#4fc3f7",
    weight: 1,
    opacity: 1,
    fillOpacity: 0.7,
};

const stylePast = {
    radius: 7,
    fillColor: "#9aa0a6",
    color: "#7f7f7f",
    weight: 1,
    opacity: 0.9,
    fillOpacity: 0.6,
};

function ensureMarker(key, latlng, label) {
    let entry = locationMarkers.get(key);
    if (entry?.marker) {
        if (latlng) {
            entry.marker.setLatLng(latlng);
            entry.lastLatLng = latlng;
        }
        if (label) entry.label = label;
        return entry;
    }

    const marker = L.circleMarker(latlng, stylePast).addTo(map);
    entry = { marker, activeIds: new Set(), lastLatLng: latlng, label };
    marker.bindPopup(popupHtml(label || key, 0), { closeButton: true });
    locationMarkers.set(key, entry);
    return entry;
}

function updateMarkerUi(key) {
    const entry = locationMarkers.get(key);
    if (!entry?.marker) return;

    const activeCount = entry.activeIds.size;
    const label = entry.label || key;

    entry.marker.setStyle(activeCount > 0 ? styleCurrent : stylePast);
    entry.marker.setPopupContent(popupHtml(label, activeCount));
}

function handlePast(msg) {
    // msg: { type:"past", key, lat, lng, city, country, past:true }
    if (!msg?.key || typeof msg.lat !== "number" || typeof msg.lng !== "number") return;

    const label = msg.city || msg.key;
    const entry = ensureMarker(msg.key, [msg.lat, msg.lng], label);

    // past marker must never show counts
    entry.activeIds.clear();
    updateMarkerUi(msg.key);
}

function handleConnect(msg) {
    // msg: { type:"connect", id, key, lat, lng, city, country, connected_at }
    if (!msg?.id || !msg?.key || typeof msg.lat !== "number" || typeof msg.lng !== "number") return;

    const id = String(msg.id);
    const key = String(msg.key);
    const latlng = [msg.lat, msg.lng];

    // If user moved keys, remove from old
    const prevKey = userToLocationKey.get(id);
    if (prevKey && prevKey !== key) {
        const prev = locationMarkers.get(prevKey);
        if (prev) {
            prev.activeIds.delete(id);
            updateMarkerUi(prevKey);
        }
    }

    const label = msg.city || key;
    const entry = ensureMarker(key, latlng, label);

    entry.activeIds.add(id);
    entry.marker.setLatLng(latlng);
    entry.lastLatLng = latlng;

    userToLocationKey.set(id, key);
    updateMarkerUi(key);
}

function handleDisconnect(msg) {
    // msg: { type:"disconnect", id, key }
    if (!msg?.id) return;

    const id = String(msg.id);
    const key = String(msg.key || userToLocationKey.get(id) || "");

    if (key) {
        const entry = locationMarkers.get(key);
        if (entry) {
            entry.activeIds.delete(id);
            updateMarkerUi(key);
        }
    }

    userToLocationKey.delete(id);
}

function connectWS() {
    const protocol = window.location.protocol === "https:" ? "wss" : "ws";
    const wsUrl = `${protocol}://${window.location.host}/ws/tcp?role=map`;

    socket = new WebSocket(wsUrl);

    socket.onopen = () => console.log("Map WS connected:", wsUrl);

    socket.onmessage = (event) => {
        let msg;
        try {
            msg = JSON.parse(event.data);
        } catch {
            return;
        }

        // your server sends WsMsg with `type`
        switch (msg.type) {
            case "past":
                handlePast(msg);
                break;
            case "connect":
                handleConnect(msg);
                break;
            case "disconnect":
                handleDisconnect(msg);
                break;
            default:
                // backward compatibility: if you ever send plain {id,lat,lng,...}
                if (msg && typeof msg.id !== "undefined" && typeof msg.lat === "number" && typeof msg.lng === "number") {
                    handleConnect({...msg, type: "connect", key: msg.key || `${msg.city},${msg.country}`});
                }
                break;
        }
    };

    socket.onclose = () => {
        console.log("Map WS closed, reconnecting...");
        setTimeout(() => {
            if (map) {
                map.invalidateSize();
            }
        }, 300);
    };

    socket.onerror = () => {
        console.warn("Map WS error, closing...");
        try {
            socket.close();
        } catch {
        }
    };
}

function initMap() {
    const switzerlandBounds = [
        [45.817995, 5.955911],
        [47.808464, 10.49205],
    ];

    map = L.map("map");
    map.fitBounds(switzerlandBounds, {padding: [20, 20]});

    L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
        maxZoom: 19,
        attribution: "© OpenStreetMap contributors",
    }).addTo(map);

    L.circleMarker([46.952152, 7.43786], {
        radius: 10,
        fillColor: "#ff0000",
        color: "#ff0000",
        weight: 2,
        opacity: 1,
        fillOpacity: 0.8,
    })
        .addTo(map)
        .bindPopup("vögeli");

    setTimeout(() => map.invalidateSize(), 100);
}

window.__map_initialized = false;

window.initLeafletMap = function () {
    if (window.__map_initialized) return;
    window.__map_initialized = true;

    initMap();
    connectWS();

    // Critical: force correct sizing after layout
    setTimeout(() => {
        if (map) map.invalidateSize();
    }, 100);
};