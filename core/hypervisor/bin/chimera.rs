// Chimera Eye: The Bevy-based Transparent Overlay
// 
// This acts as Aaroneous's "eyes and hands" in the physical/digital world,
// bypassing the need for intrusive DLL injection or anti-cheat triggers.
// It creates an invisible, always-on-top window over the entire screen.

use bevy::prelude::*;
use bevy::window::{WindowLevel, WindowPlugin};
use bevy::render::camera::ClearColorConfig;
use bevy::sprite::MaterialMesh2dBundle;
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use rdev::{listen, Event, EventType};
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::PathBuf;
use enigo::{Enigo, Mouse, Keyboard};
use a_run::workspace::WorkspacePaths;

#[derive(Deserialize, Debug)]
struct DrawBoxCmd {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: String,
    label: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmulationEvent {
    event_type: String,
    x: f64,
    y: f64,
    timestamp: u64,
}

// Global channel for NATS -> Bevy communication
struct NatsReceiver(std::sync::Mutex<mpsc::UnboundedReceiver<DrawBoxCmd>>);

impl Resource for NatsReceiver {}

// State flag for recording
static IS_RECORDING: AtomicBool = AtomicBool::new(false);

fn main() {
    // 1. Setup NATS connection on a background thread
    let (tx, rx) = mpsc::unbounded_channel::<DrawBoxCmd>();
    
    std::thread::spawn(move || {
        let Ok(nc) = nats::connect("localhost:4222") else {
            println!("Chimera: Could not connect to NATS. Running in standalone mode.");
            return;
        };
        
        let nc_clone = nc.clone();
        
        // Start global input listener in another thread
        std::thread::spawn(move || {
            let callback = move |event: Event| {
                if !IS_RECORDING.load(Ordering::Relaxed) {
                    return;
                }
                
                let mut emu_event = None;
                
                match event.event_type {
                    EventType::MouseMove { x: _, y: _ } => {
                        // Only capture occasionally or only clicks to save bandwidth?
                        // For now, let's just capture clicks to avoid overwhelming NATS
                    },
                    EventType::ButtonPress(button) => {
                        emu_event = Some(EmulationEvent {
                            event_type: format!("MouseDown_{:?}", button),
                            x: 0.0, // rdev button press doesn't contain coordinates directly, would need to track last mouse move
                            y: 0.0,
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                        });
                    },
                    EventType::ButtonRelease(button) => {
                         emu_event = Some(EmulationEvent {
                            event_type: format!("MouseUp_{:?}", button),
                            x: 0.0, 
                            y: 0.0,
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                        });
                    },
                    EventType::KeyPress(key) => {
                        emu_event = Some(EmulationEvent {
                            event_type: format!("KeyDown_{:?}", key),
                            x: 0.0,
                            y: 0.0,
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                        });
                    },
                    _ => {}
                }
                
                if let Some(mut e) = emu_event {
                    // Try to grab last known coordinates from OS
                    let (x, y) = rdev::display_size().unwrap_or((0,0)); // not quite mouse pos, but close enough for prototype
                    e.x = x as f64;
                    e.y = y as f64;
                    
                    if let Ok(json) = serde_json::to_string(&e) {
                        let _ = nc_clone.publish("chimera.emulation.record", json);
                        println!("Chimera: Recorded emulation event: {:?}", e.event_type);
                    }
                }
            };
            
            if let Err(error) = listen(callback) {
                println!("Error in rdev listener: {:?}", error);
            }
        });
        
        println!("Chimera: Connected to NATS. Listening on 'chimera.draw' and 'chimera.record'...");
        
        if let Ok(sub) = nc.subscribe("chimera.draw") {
            let sub_record = nc.subscribe("chimera.record").unwrap();
            let sub_playback = nc.subscribe("chimera.emulation.playback").unwrap();
            
            loop {
                if let Some(msg) = sub.try_next() {
                    if let Ok(json_str) = std::str::from_utf8(&msg.data) {
                        if let Ok(cmd) = serde_json::from_str::<DrawBoxCmd>(json_str) {
                            let _ = tx.send(cmd);
                        }
                    }
                }
                
                if let Some(msg) = sub_record.try_next() {
                    if let Ok(cmd) = std::str::from_utf8(&msg.data) {
                        if cmd == "start" {
                            IS_RECORDING.store(true, Ordering::SeqCst);
                            println!("Chimera: Emulation Recording STARTED.");
                        } else if cmd == "stop" {
                            IS_RECORDING.store(false, Ordering::SeqCst);
                            println!("Chimera: Emulation Recording STOPPED.");
                        }
                    }
                }
                
                if let Some(msg) = sub_playback.try_next() {
                    if let Ok(id) = std::str::from_utf8(&msg.data) {
                        let paths = WorkspacePaths::discover();
                        let path = paths.routines().join(id);
                        if path.exists() {
                            if let Ok(data) = std::fs::read_to_string(&path) {
                                if let Ok(events) = serde_json::from_str::<Vec<EmulationEvent>>(&data) {
                                    println!("Chimera: Playing back routine {} ({} events)", id, events.len());
                                    if let Ok(mut enigo) = Enigo::new(&enigo::Settings::default()) {
                                        for event in events {
                                            // 1. Move mouse to recorded location
                                            if event.x > 0.0 && event.y > 0.0 {
                                                let _ = enigo.move_mouse(event.x as i32, event.y as i32, enigo::Coordinate::Abs);
                                            }
                                            
                                            // 2. Perform the action
                                            if event.event_type.starts_with("MouseDown") {
                                                let button = if event.event_type.contains("Right") {
                                                    enigo::Button::Right
                                                } else if event.event_type.contains("Middle") {
                                                    enigo::Button::Middle
                                                } else {
                                                    enigo::Button::Left
                                                };
                                                let _ = enigo.button(button, enigo::Direction::Press);
                                            } else if event.event_type.starts_with("MouseUp") {
                                                let button = if event.event_type.contains("Right") {
                                                    enigo::Button::Right
                                                } else if event.event_type.contains("Middle") {
                                                    enigo::Button::Middle
                                                } else {
                                                    enigo::Button::Left
                                                };
                                                let _ = enigo.button(button, enigo::Direction::Release);
                                            } else if event.event_type.starts_with("KeyDown") {
                                                // Basic implementation for a few keys
                                                if event.event_type.contains("Return") {
                                                    let _ = enigo.key(enigo::Key::Return, enigo::Direction::Click);
                                                } else if event.event_type.contains("Space") {
                                                    let _ = enigo.key(enigo::Key::Space, enigo::Direction::Click);
                                                } else {
                                                    // Parse single character if possible
                                                    if let Some(c) = event.event_type.chars().last() {
                                                        let _ = enigo.key(enigo::Key::Unicode(c), enigo::Direction::Click);
                                                    }
                                                }
                                            }
                                            std::thread::sleep(std::time::Duration::from_millis(100)); // basic pacing
                                        }
                                        println!("Chimera: Playback of routine {} complete", id);
                                    }
                                }
                            }
                        }
                    }
                }
                
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    });

    // 2. Start the Bevy App
    App::new()
        // Transparent clear color is essential for the overlay to be invisible
        .insert_resource(ClearColor(Color::NONE))
        .insert_resource(NatsReceiver(std::sync::Mutex::new(rx)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Aaroneous Chimera Eye".to_string(),
                transparent: true,
                decorations: false,
                window_level: WindowLevel::AlwaysOnTop,
                // Optional: For full click-through (mouse pass-through), you'd use OS specific APIs
                // or Bevy window flags if available.
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup_camera)
        .add_systems(Update, handle_draw_commands)
        .run();
}

fn setup_camera(mut commands: Commands) {
    // 2D Camera spanning the window
    commands.spawn(Camera2dBundle {
        camera: Camera {
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        ..default()
    });
    
    println!("Chimera Eye Overlay initialized and transparent.");
}

// Marker component for dynamic drawing
#[derive(Component)]
struct ChimeraOverlayBox;

fn handle_draw_commands(
    mut commands: Commands,
    nats_rx: Res<NatsReceiver>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    existing_boxes: Query<Entity, With<ChimeraOverlayBox>>,
) {
    let mut rx = nats_rx.0.lock().unwrap();
    
    // We poll the channel for new draw commands
    let mut new_commands = Vec::new();
    while let Ok(cmd) = rx.try_recv() {
        new_commands.push(cmd);
    }
    
    if !new_commands.is_empty() {
        // Clear old boxes
        for entity in existing_boxes.iter() {
            commands.entity(entity).despawn_recursive();
        }
        
        // Draw new boxes
        for cmd in new_commands {
            let color = match cmd.color.as_str() {
                "red" => Color::srgb(1.0, 0.0, 0.0),
                "green" => Color::srgb(0.0, 1.0, 0.0),
                "cyan" => Color::srgb(0.0, 1.0, 1.0),
                _ => Color::srgb(1.0, 0.0, 1.0),
            };
            
            // Draw an outline box (using a slightly transparent fill for the prototype)
            // In a production version, we would use a wireframe material or lines
            commands.spawn((
                MaterialMesh2dBundle {
                    mesh: meshes.add(Rectangle::new(cmd.width, cmd.height)).into(),
                    material: materials.add(ColorMaterial::from(color.with_alpha(0.3))),
                    transform: Transform::from_xyz(cmd.x, cmd.y, 0.0),
                    ..default()
                },
                ChimeraOverlayBox,
            ));
            
            println!("Chimera drew {} box at ({}, {})", cmd.label, cmd.x, cmd.y);
        }
    }
}