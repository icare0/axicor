use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    render::render_asset::RenderAssetUsages,
};
use bevy_egui::egui;
use egui_tiles::{Tile, Container, Linear, SimplificationOptions};

use crate::layout::domain::{Pane, WorkspaceState, TreeCommands};
use layout_api::{PluginWindow, TreeCommand, TopologyCache, base_domain, domain_title};

// ---------------------------------------------------------------------------

pub fn execute_window_commands_system(
    mut commands: Commands,
    mut workspace: ResMut<WorkspaceState>,
    mut commands_queue: ResMut<TreeCommands>,
    topology: Res<TopologyCache>,
    mut images: ResMut<Assets<Image>>,
    time: Res<Time>,
) {
    let active_ws = workspace.active_workspace.clone();
    if let Some(tree) = workspace.trees.get_mut(&active_ws) {
        for cmd in commands_queue.queue.drain(..) {
            match cmd {
                TreeCommand::Split { target, axis, fraction, insert_before, plugin_id } =>
                    handle_split(&mut commands, tree, &topology, &mut images, &time,
                                target, axis, fraction, insert_before, &plugin_id),

                TreeCommand::Merge { survivor, victim } =>
                    handle_merge(tree, survivor, victim),

                TreeCommand::SwapPanes { src, dst } =>
                    handle_swap(tree, src, dst),

                TreeCommand::ChangeDomain { tile_id, new_domain } =>
                    handle_change_domain(tree, tile_id, &new_domain),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

fn handle_split(
    commands: &mut Commands,
    tree: &mut egui_tiles::Tree<Pane>,
    topology: &TopologyCache,
    images: &mut Assets<Image>,
    time: &Time,
    target: egui_tiles::TileId,
    axis: egui_tiles::LinearDir,
    fraction: f32,
    insert_before: bool,
    plugin_id: &str,
) {
    let rect = topology.tiles.get(&target).copied().unwrap_or(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)));
    let base = base_domain(plugin_id);
    //  ID  elapsed    ECS-  SystemTime
    let new_plugin_id = format!("{}::{}", base, time.elapsed().as_micros());
    let pane = Pane { plugin_id: new_plugin_id.clone(), title: domain_title(base).to_string() };

    spawn_pane_entity(commands, images, &new_plugin_id, rect.width(), rect.height());

    let Some(Tile::Pane(old_pane)) = tree.tiles.get(target).cloned() else { return };

    let old_id = tree.tiles.insert_pane(old_pane);
    let new_id = tree.tiles.insert_pane(pane);

    let (children, old_share, new_share) = if insert_before {
        (vec![new_id, old_id], 1.0 - fraction, fraction)
    } else {
        (vec![old_id, new_id], fraction, 1.0 - fraction)
    };

    let mut linear = Linear { dir: axis, children, ..default() };
    linear.shares.set_share(old_id, old_share);
    linear.shares.set_share(new_id, new_share);
    tree.tiles.insert(target, Tile::Container(Container::Linear(linear)));
}

fn handle_merge(tree: &mut egui_tiles::Tree<Pane>, survivor: egui_tiles::TileId, victim: egui_tiles::TileId) {
    // O(N)  
    let parent_map = build_parent_map(&tree.tiles);

    let Some(&victim_parent) = parent_map.get(&victim) else { return };

    //  survivor    
    let mut survivor_branch = survivor;
    while let Some(&p) = parent_map.get(&survivor_branch) {
        if p == victim_parent { break; }
        survivor_branch = p;
    }

    //  share   survivor  
    if let Some(Tile::Container(Container::Linear(lin))) = tree.tiles.get_mut(victim_parent) {
        let v_share = lin.shares[victim];
        let s_share = lin.shares[survivor_branch];
        lin.shares.set_share(survivor_branch, s_share + v_share);
        lin.children.retain(|&c| c != victim);
    }

    tree.tiles.remove(victim);
    tree.simplify(&SimplificationOptions { all_panes_must_have_tabs: false, ..default() });
}

fn handle_swap(tree: &mut egui_tiles::Tree<Pane>, src: egui_tiles::TileId, dst: egui_tiles::TileId) {
    let (Some(Tile::Pane(src_pane)), Some(Tile::Pane(dst_pane))) = (
        tree.tiles.get(src).cloned(),
        tree.tiles.get(dst).cloned(),
    ) else { return };

    if let Some(Tile::Pane(p)) = tree.tiles.get_mut(src) { *p = dst_pane; }
    if let Some(Tile::Pane(p)) = tree.tiles.get_mut(dst) { *p = src_pane; }
}

fn handle_change_domain(tree: &mut egui_tiles::Tree<Pane>, tile_id: egui_tiles::TileId, new_domain: &str) {
    let base = base_domain(new_domain);
    let pane = Pane { plugin_id: new_domain.to_string(), title: domain_title(base).to_string() };
    if let Some(Tile::Pane(p)) = tree.tiles.get_mut(tile_id) {
        *p = pane;
    }
}

fn build_parent_map(tiles: &egui_tiles::Tiles<Pane>) -> bevy::utils::HashMap<egui_tiles::TileId, egui_tiles::TileId> {
    let mut map = bevy::utils::HashMap::new();
    for (id, tile) in tiles.iter() {
        let children: &[egui_tiles::TileId] = match tile {
            Tile::Container(Container::Linear(l)) => &l.children,
            Tile::Container(Container::Tabs(t))   => &t.children,
            _ => continue,
        };
        for &child in children {
            map.insert(child, *id);
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

pub fn spawn_pane_entity(
    commands: &mut Commands,
    _images: &mut Assets<Image>,
    plugin_id: &str,
    width: f32,
    height: f32,
) -> Entity {
    // DOD FIX: WM    . 
    //     sync_plugin_geometry_system.
    //          (, Camera3d).
    commands.spawn((
        PluginWindow { 
            plugin_id: plugin_id.to_string(), 
            texture: None, 
            is_visible: true,
            id: egui::Id::new(plugin_id),
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height)),
        },
        layout_api::PluginInput::default(),
        layout_api::PluginGeometry { size: Vec2::new(width, height) },
    )).id()
}

pub fn create_plugin_render_target(images: &mut Assets<Image>, width: u32, height: u32) -> Handle<Image> {
    let size = Extent3d { width, height, depth_or_array_layers: 1 };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0; 4],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
    images.add(image)
}
