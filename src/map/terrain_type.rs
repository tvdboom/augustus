//! Broad landscape portraits for the province overview. These are display
//! classifications, independent of the map's geographic landcover texture.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerrainType {
    Desert,
    Farmland,
    Forest,
    Hills,
    Marsh,
    Mountains,
    Plains,
}

impl TerrainType {
    pub(crate) fn image_index(self) -> usize {
        match self {
            Self::Desert => 0,
            Self::Farmland => 1,
            Self::Forest => 2,
            Self::Hills => 3,
            Self::Marsh => 6,
            Self::Mountains => 7,
            Self::Plains => 8,
        }
    }
}

pub(crate) fn for_province(name: &str) -> TerrainType {
    use TerrainType::*;
    match name {
        "Aegyptus"
        | "Arabia"
        | "Cyrenaica"
        | "Numidia"
        | "Mauretania Caesariensis"
        | "Mauretania Tingitana" => Desert,
        "Latium"
        | "Aemilia"
        | "Apulia"
        | "Baetica"
        | "Sicilia"
        | "Transpadana"
        | "Venetia"
        | "Africa Proconsularis"
        | "Asia" => Farmland,
        "Britannia" | "Belgica" | "Germania Inferior" | "Germania Superior" | "Lugdunensis"
        | "Aquitania" | "Dacia" => Forest,
        "Alpes Graiae"
        | "Alpes Cottiae"
        | "Alpes Maritimae"
        | "Raetia"
        | "Noricum"
        | "Armenia Mesopotamia" => Mountains,
        "Sardinia" | "Creta" | "Cyprus" | "Iudaea" | "Moesia Inferior" => Plains,
        "Pannonia Inferior" => Marsh,
        "Liguria" | "Narbonensis" | "Tarraconensis" | "Lusitania" | "Etruria" | "Umbria"
        | "Samnium" | "Picenum" | "Lucania" | "Pannonia Superior" | "Dalmatia"
        | "Moesia Superior" | "Thracia" | "Macedonia" | "Achaia" | "Bithynia" | "Cilicia"
        | "Syria" | "Galatia" | "Lycia" => Hills,
        _ => panic!("province {name} has no terrain portrait"),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_playable_province_has_a_terrain_portrait() {
        for (name, _) in crate::map::production::OUTPUT {
            let terrain = super::for_province(name);
            assert!(terrain.image_index() < 12, "{name}");
        }
    }
}
