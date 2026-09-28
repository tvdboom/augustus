//! Explicit sea crossings shared by routefinding and the map lines.

/// A traversable strait or sea lane, with geographic shore anchors.
pub(super) struct SeaCrossing {
    /// Canonical atlas province names.
    pub provinces: [&'static str; 2],
    /// Longitude/latitude endpoints, placed at the relevant coasts.
    pub shores: [[f32; 2]; 2],
}

/// User-specified bidirectional links; these are ordinary graph edges, not naval combat.
pub(super) const SEA_CROSSINGS: [SeaCrossing; 8] = [
    SeaCrossing {
        provinces: ["Asia", "Achaia"],
        shores: [[26.7, 38.4], [24.0, 38.0]],
    },
    SeaCrossing {
        provinces: ["Creta", "Achaia"],
        shores: [[23.58, 35.54], [23.01, 36.55]],
    },
    SeaCrossing {
        provinces: ["Creta", "Asia"],
        // Connect eastern Crete to the southwest coast of Rhodos, within Asia.
        shores: [[26.26, 35.21], [27.76, 35.95]],
    },
    SeaCrossing {
        provinces: ["Sicilia", "Africa Proconsularis"],
        shores: [[12.5, 37.8], [10.9, 37.0]],
    },
    SeaCrossing {
        provinces: ["Africa Proconsularis", "Sardinia"],
        shores: [[9.9, 37.2], [9.1, 39.0]],
    },
    SeaCrossing {
        provinces: ["Sardinia", "Etruria"],
        shores: [[9.5, 41.1], [11.0, 42.4]],
    },
    SeaCrossing {
        provinces: ["Mauretania Tingitana", "Baetica"],
        shores: [[-5.5, 35.7], [-5.6, 36.1]],
    },
    SeaCrossing {
        provinces: ["Britannia", "Belgica"],
        shores: [[1.3, 51.1], [2.0, 50.9]],
    },
];
