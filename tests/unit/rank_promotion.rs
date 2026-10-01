use super::*;

#[test]
fn both_rank_styles_join_their_emblem_and_leave_no_container_or_expired_shapes() {
    for rank in
        [Rank::Military(MilitaryRank::MilitaryTribune), Rank::Political(PoliticalRank::Aedile)]
    {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(960.0, 720.0));
        for time in [0.4, 1.05, 1.5, 2.5, 3.55, DURATION] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| paint(ui.ctx(), rank, time, 1.0),
            );
            assert!(
                !output.shapes.iter().any(|shape| matches!(shape.shape, egui::Shape::Rect(_))),
                "promotion must have no enclosing card"
            );
            if time >= DURATION {
                assert!(output.shapes.is_empty(), "expired effects must disappear entirely");
            } else {
                let art = textures(&ctx, rank);
                let pieces: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Mesh(mesh) if mesh.texture_id == art.color.id() => Some(mesh),
                        _ => None,
                    })
                    .collect();
                assert_eq!(pieces.len(), 8);
                if time == 2.5 {
                    let target =
                        egui::Rect::from_center_size(screen.center(), egui::Vec2::splat(224.0));
                    for piece in pieces {
                        for vertex in &piece.vertices {
                            let expected = target.min + vertex.uv.to_vec2() * target.size();
                            assert!(
                                vertex.pos.distance(expected) < 0.001,
                                "all assembled seams must join exactly"
                            );
                        }
                    }
                }
            }
            output.textures_delta.clear();
        }
    }
}

/// Export actual egui meshes for optional visual inspection without a GPU/window.
#[test]
#[ignore = "set AUGUSTUS_RANK_PREVIEW to a directory to export rendered animation frames"]
fn export_rank_animation_preview() {
    let directory = std::path::PathBuf::from(
        std::env::var_os("AUGUSTUS_RANK_PREVIEW").expect("preview directory"),
    );
    std::fs::create_dir_all(&directory).unwrap();
    for rank in [
        Rank::Military(MilitaryRank::MilitaryTribune),
        Rank::Military(MilitaryRank::Imperator),
        Rank::Political(PoliticalRank::Aedile),
        Rank::Political(PoliticalRank::Augustus),
    ] {
        for (frame, time) in [0.3, 0.55, 1.05, 1.5, 2.5, 3.55].into_iter().enumerate() {
            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(960.0, 720.0),
                    )),
                    ..Default::default()
                },
                |ui| paint(ui.ctx(), rank, time, 1.0),
            );
            let prefix = format!("{}-{frame}", rank.key());
            let mut texture_files = Vec::new();
            for (id, deltas) in &output.textures_delta.set {
                let mut pixels = image::RgbaImage::new(1, 1);
                for delta in deltas {
                    let egui::ImageData::Color(image) = &delta.image;
                    let bytes: Vec<_> =
                        image.pixels.iter().flat_map(|color| color.to_array()).collect();
                    let update = image::RgbaImage::from_raw(
                        image.size[0] as u32,
                        image.size[1] as u32,
                        bytes,
                    )
                    .unwrap();
                    if let Some(pos) = delta.pos {
                        image::imageops::overlay(
                            &mut pixels,
                            &update,
                            pos[0] as i64,
                            pos[1] as i64,
                        );
                    } else {
                        pixels = update;
                    }
                }
                let file = format!("{prefix}-{id:?}.png");
                pixels.save(directory.join(&file)).unwrap();
                texture_files.push(serde_json::json!({"id": format!("{id:?}"), "file": file}));
            }
            let meshes: Vec<_> = ctx.tessellate(output.shapes, output.pixels_per_point).into_iter()
                .filter_map(|primitive| match primitive.primitive {
                    egui::epaint::Primitive::Mesh(mesh) => Some(serde_json::json!({
                        "texture": format!("{:?}", mesh.texture_id), "indices": mesh.indices,
                        "vertices": mesh.vertices.iter().map(|v| serde_json::json!({
                            "pos": [v.pos.x, v.pos.y], "uv": [v.uv.x, v.uv.y], "color": v.color.to_array()
                        })).collect::<Vec<_>>(),
                    })),
                    _ => None,
                }).collect();
            std::fs::write(
                directory.join(format!("{prefix}.json")),
                serde_json::to_vec(&serde_json::json!({
                    "textures": texture_files, "meshes": meshes,
                }))
                .unwrap(),
            )
            .unwrap();
            output.textures_delta.clear();
        }
    }
}
