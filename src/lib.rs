use bevy::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Resource, Default)]
struct WatchedDirs {
    dirs: Vec<PathBuf>,
    imgs: Vec<PathBuf>,
}

#[derive(Component)]
struct ImageMarker {
    target: PathBuf,
}

pub struct DirWatchingPlugin;

impl Plugin for DirWatchingPlugin {
    fn build(&self, app: &mut App) {
        log::debug!("Adding DirWatchingPlugin");
        // Probs do this for yours:
        // app.insert_resource(WatchedDirs::default());

        // for demo purposes
        let img_dirs_testing = PathBuf::from("/media/jer/ARCHIVE/jpg/2024/December");
        app.insert_resource(WatchedDirs {
            dirs: vec![img_dirs_testing],
            imgs: vec![],
        });

        app.add_systems(PreUpdate, scan_directories_system);
        // app.add_systems(Update, spawn_img_paths.run_if(WatchedDirs::should_run));
        app.add_systems(Update, slap_img_on_quad.run_if(WatchedDirs::should_run));
    }
}

/// System that handles directory scanning
fn scan_directories_system(
    mut watched_dirs: ResMut<WatchedDirs>,
    time: Res<Time>,
    mut last_scan: Local<Option<f32>>,
) {
    // Only scan every 5 seconds to avoid performance hits
    let scan_interval = 5.0;

    if let Some(last) = *last_scan
        && time.elapsed_secs() - last < scan_interval {
            return;
        }

    watched_dirs.scan();
    *last_scan = Some(time.elapsed_secs());
}

impl WatchedDirs {
    fn should_run(res: Res<WatchedDirs>) -> bool {
        !res.imgs.is_empty()
    }

    /// Supported image extensions
    const SUPPORTED_EXTENSIONS: &'static [&'static str] = &[
        "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp", "ico", "svg",
    ];

    /// Check if a file has a supported image extension
    fn is_supported_image(path: &Path) -> bool {
        if let Some(extension) = path.extension()
            && let Some(ext_str) = extension.to_str() {
                return Self::SUPPORTED_EXTENSIONS.contains(&ext_str.to_lowercase().as_str());
            }
        false
    }

    /// Recursively collect all image files from a directory
    fn collect_images_recursive(
        dir: &Path,
        images: &mut Vec<PathBuf>,
    ) -> Result<(), std::io::Error> {
        if !dir.is_dir() {
            return Ok(());
        }

        let entries = fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively scan subdirectories
                Self::collect_images_recursive(&path, images)?;
            } else if path.is_file() && Self::is_supported_image(&path) {
                images.push(path);
            }
        }
        Ok(())
    }

    /// Scan all directories and populate the imgs vector with found image files
    fn scan(&mut self) {
        self.imgs.clear();

        for dir in &self.dirs {
            if dir.exists() {
                if let Err(e) = Self::collect_images_recursive(dir, &mut self.imgs) {
                    log::warn!("Error scanning directory {dir:?}: {e}");
                }
            } else {
                log::warn!("Directory does not exist: {dir:?}");
            }
        }

        log::debug!(
            "Found {} images across {} directories",
            self.imgs.len(),
            self.dirs.len()
        );
    }
}

fn slap_img_on_quad(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    watched_dirs: Res<WatchedDirs>,
    existing_quads: Query<&ImageMarker>,
) {
    let existing_paths: std::collections::HashSet<&Path> = existing_quads
        .iter()
        .map(|marker| marker.target.as_path())
        .collect();

    // Grid configuration
    let grid_size = (watched_dirs.imgs.len() as f32).sqrt().ceil() as i32;
    let quad_spacing = 2.5f32;
    let quad_size = 2.0f32;

    // Create mesh for the quad (using Rectangle shape)
    let quad_mesh = meshes.add(Rectangle::new(quad_size, quad_size));

    let mut spawned_count = 0;

    // Spawn quads for new images
    for (index, img_path) in watched_dirs.imgs.iter().enumerate() {
        if !existing_paths.contains(img_path.as_path()) {
            // Calculate grid position
            let grid_pos = calculate_grid_position(index, grid_size, quad_spacing);

            // Load the image as a texture
            let texture_handle: Handle<Image> =
                asset_server.load(img_path.to_string_lossy().to_string());

            // asset_server.load(format!("file://{}", img_path.to_string_lossy()));

            // Create material with the image texture
            let material = materials.add(StandardMaterial {
                base_color_texture: Some(texture_handle),
                unlit: true,
                ..default()
            });

            // Spawn the quad entity with modern component-based approach
            commands.spawn((
                Mesh3d(quad_mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_translation(grid_pos),
                // .looking_at(Vec3::ZERO, Vec3::Y),
                ImageMarker {
                    target: img_path.clone(),
                },
                // Visibility::default(),
                // InheritedVisibility::default(),
                ViewVisibility::default(),
            ));

            spawned_count += 1;
            log::debug!(
                "Spawned quad for image: {img_path:?} at position: {grid_pos:?}"
            );
        }
    }

    if spawned_count > 0 {
        log::info!(
            "Spawned {spawned_count} image quads in a {grid_size}x{grid_size} grid"
        );
    }
}

/// Helper function to calculate grid position for an image quad
fn calculate_grid_position(index: usize, grid_size: i32, spacing: f32) -> Vec3 {
    let row = (index as i32) / grid_size;
    let col = (index as i32) % grid_size;

    // Center the grid around origin
    let offset_x = (grid_size as f32 - 1.0) * spacing * 0.5;
    let offset_z = (grid_size as f32 - 1.0) * spacing * 0.5;

    Vec3::new(
        (col as f32 * spacing) - offset_x,
        0.0, // small bump
        (row as f32 * spacing) - offset_z,
    )
}
