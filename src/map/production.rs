//! Base raw-resource potential for the historical map. Province population
//! scales these values into monthly output. Geography and known grain regions,
//! mines, and quarries inform the relative strengths; these are game balance
//! units, not historical yield estimates.

pub(crate) const PRODUCTION_ICONS: [&[u8]; 3] = [
    include_bytes!("../../assets/images/icons/food.png"),
    include_bytes!("../../assets/images/icons/metal.png"),
    include_bytes!("../../assets/images/icons/stone.png"),
];

// Food (grain), metal, stone. Keep this table aligned with the atlas names.
pub(super) const OUTPUT: [(&str, [i32; 3]); 54] = [
    ("Britannia", [2, 3, 1]),
    ("Lugdunensis", [3, 1, 1]),
    ("Belgica", [3, 1, 0]),
    ("Germania Inferior", [2, 2, 1]),
    ("Aquitania", [3, 1, 1]),
    ("Germania Superior", [2, 2, 2]),
    ("Alpes Graiae", [0, 2, 3]),
    ("Transpadana", [4, 1, 2]),
    ("Alpes Cottiae", [0, 2, 3]),
    ("Alpes Maritimae", [1, 2, 3]),
    ("Liguria", [2, 2, 2]),
    ("Narbonensis", [3, 1, 2]),
    ("Tarraconensis", [3, 4, 2]),
    ("Baetica", [4, 4, 1]),
    ("Lusitania", [2, 4, 2]),
    ("Raetia", [1, 3, 2]),
    ("Noricum", [1, 5, 2]),
    ("Venetia", [3, 1, 1]),
    ("Aemilia", [4, 0, 1]),
    ("Etruria", [3, 3, 3]),
    ("Umbria", [3, 1, 2]),
    ("Samnium", [2, 2, 2]),
    ("Picenum", [3, 0, 1]),
    ("Latium", [3, 0, 2]),
    ("Lucania", [2, 1, 2]),
    ("Sicilia", [5, 1, 2]),
    ("Pannonia Superior", [2, 3, 1]),
    ("Pannonia Inferior", [2, 2, 1]),
    ("Dalmatia", [2, 3, 3]),
    ("Apulia", [4, 0, 1]),
    ("Sardinia", [3, 3, 2]),
    ("Moesia Superior", [2, 3, 2]),
    ("Dacia", [2, 5, 2]),
    ("Moesia Inferior", [4, 1, 1]),
    ("Thracia", [3, 3, 2]),
    ("Macedonia", [3, 3, 2]),
    ("Achaia", [2, 1, 4]),
    ("Bithynia", [3, 1, 3]),
    ("Cilicia", [2, 2, 2]),
    ("Cyrenaica", [2, 0, 2]),
    ("Creta", [2, 0, 2]),
    ("Cyprus", [2, 4, 2]),
    ("Aegyptus", [6, 1, 4]),
    ("Arabia", [0, 1, 2]),
    ("Iudaea", [2, 0, 2]),
    ("Syria", [3, 1, 2]),
    ("Africa Proconsularis", [5, 0, 3]),
    ("Numidia", [3, 1, 3]),
    ("Mauretania Caesariensis", [2, 2, 2]),
    ("Mauretania Tingitana", [2, 2, 2]),
    ("Galatia", [3, 2, 2]),
    ("Lycia", [2, 1, 4]),
    ("Asia", [4, 1, 4]),
    ("Armenia Mesopotamia", [3, 4, 2]),
];

pub(super) fn for_province(name: &str) -> [i32; 3] {
    OUTPUT
        .iter()
        .find(|(province, _)| *province == name)
        .unwrap_or_else(|| panic!("province {name} has no production values"))
        .1
}
