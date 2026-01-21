let map;
const locationMarkers = new Map(); // key -> { marker, count, activeIds: Set }
const userToLocationKey = new Map(); // userId -> locationKey

(function injectPastStyles() {
    const css = [
        /* gray popup text for past locations */
        '.leaflet-popup-content .past {',
        '  color: #777;',
        '}',
        /* slightly dim current popup text a bit less */
        '.leaflet-popup-content .current {',
        '  color: #222;',
        '}'
    ].join('\n');
    const s = document.createElement('style');
    s.type = 'text/css';
    s.appendChild(document.createTextNode(css));
    document.head.appendChild(s);
})();

function locKeyForUser(user) {
    if (user.city && user.city.length) {
        // prefer city + country if available
        return user.country ? `${user.city}, ${user.country}` : user.city;
    }
    // fallback: rounded coordinates to group nearby hits
    const lat = Math.round(user.lat * 100) / 100;
    const lng = Math.round(user.lng * 100) / 100;
    return `${lat},${lng}`;
}

function popupHtmlForLocation(label, count, hasActive) {
    const statusClass = hasActive ? 'current' : 'past';
    const plural = count === 1 ? 'user' : 'users';
    return `<div class="${statusClass}"><b>${escapeHtml(label)}</b><br/>${hasActive ? `Active users: <strong>${count}</strong>` : `Visited before`}${plural}</div>`;
}

function escapeHtml(s) {
    return String(s || '')
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
}

function createOrUpdateLocationMarker(key, latlng, isActive) {
    const entry = locationMarkers.get(key);
    const styleCurrent = {
        radius: 7,
        fillColor: "#4fc3f7",
        color: "#4fc3f7",
        weight: 1,
        opacity: 1,
        fillOpacity: 0.7
    };
    const stylePast = {
        radius: 7,
        fillColor: "#9aa0a6",
        color: "#7f7f7f",
        weight: 1,
        opacity: 0.9,
        fillOpacity: 0.6
    };

    if (entry && entry.marker) {
        // update position and style
        entry.marker.setLatLng(latlng);
        entry.marker.setStyle(isActive ? styleCurrent : stylePast);
        // update popup
        entry.marker.setPopupContent(popupHtmlForLocation(key, entry.count, isActive));
        entry.lastLatLng = latlng;
        entry.isActive = isActive;
    } else {
        // create new aggregated marker for this location
        const marker = L.circleMarker(latlng, isActive ? styleCurrent : stylePast).addTo(map);
        marker.bindPopup(popupHtmlForLocation(key, 1, isActive), { closeButton: true });
        locationMarkers.set(key, {
            marker,
            count: 1,
            activeIds: new Set(),
            lastLatLng: latlng,
            isActive
        });
    }
}

function setLocationCount(key, newCount) {
    const entry = locationMarkers.get(key);
    if (!entry) return;
    entry.count = newCount;
    entry.marker.setPopupContent(popupHtmlForLocation(key, entry.count, entry.activeIds.size > 0));
}

function ensureLocationExists(key, latlng) {
    if (!locationMarkers.has(key)) {
        // create with default inactive style (will be updated immediately after)
        createOrUpdateLocationMarker(key, latlng, false);
    } else {
        // if marker exists, ensure lastLatLng is set
        const e = locationMarkers.get(key);
        if (!e.lastLatLng && latlng) {
            e.lastLatLng = latlng;
            e.marker.setLatLng(latlng);
        }
    }
}

function upsertFromEvent(user) {
    // user expected to have: id, lat, lng, city, country, connected_at
    if (!user || typeof user.id === 'undefined') return;

    const id = String(user.id);
    const latlng = [user.lat, user.lng];
    const key = locKeyForUser(user);

    // If this user was previously mapped to a different key, remove from that previous location's active set
    const previousKey = userToLocationKey.get(id);
    if (previousKey && previousKey !== key) {
        const prev = locationMarkers.get(previousKey);
        if (prev) {
            prev.activeIds.delete(id);
            // update active/past style
            prev.marker.setStyle(prev.activeIds.size > 0 ? {
                radius: 7,
                fillColor: "#4fc3f7",
                color: "#4fc3f7",
                weight: 1,
                opacity: 1,
                fillOpacity: 0.7
            } : {
                radius: 7,
                fillColor: "#9aa0a6",
                color: "#7f7f7f",
                weight: 1,
                opacity: 0.9,
                fillOpacity: 0.6
            });
            // update popup text (count stays)
            prev.marker.setPopupContent(popupHtmlForLocation(previousKey, prev.count, prev.activeIds.size > 0));
        }
    }

    // ensure aggregated marker exists
    ensureLocationExists(key, latlng);

    // mark this user as active for this key
    const entry = locationMarkers.get(key);
    entry.activeIds.add(id);
    entry.count = (entry.count || 0) + (previousKey === key ? 0 : 1); // increment only if this is a new association for that user

    // update representative marker position to latest latlng
    createOrUpdateLocationMarker(key, latlng, entry.activeIds.size > 0);

    // store mapping
    userToLocationKey.set(id, key);
}


function initMap() {
    const switzerlandBounds = [
        [45.817995, 5.955911],
        [47.808464, 10.49205]
    ];

    map = L.map("map");
    map.fitBounds(switzerlandBounds, { padding: [20, 20] });

    L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
        maxZoom: 19,
        attribution: "© OpenStreetMap contributors"
    }).addTo(map);

    L.circleMarker([46.952152, 7.437860], {
        radius: 10,
        fillColor: "#ff0000",
        color: "#ff0000",
        weight: 2,
        opacity: 1,
        fillOpacity: 0.8
    })
        .addTo(map)
        .bindPopup("vögeli");

    setTimeout(() => {
        map.invalidateSize();
    }, 0);
}


function onReady(fn) {
    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", fn);
    } else {
        fn();
    }
}

onReady(() => {
    initMap();
});