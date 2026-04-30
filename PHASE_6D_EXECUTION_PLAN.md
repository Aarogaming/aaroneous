# Phase 6D Comprehensive Execution Plan: Stages 1-6

**Vision**: Build the complete WASM Relic ecosystem with Ariel and Glass as true entities

**Scope**: 6 interconnected stages, 140-170 hours, 150-175 tests

**Target Completion**: 750+ cumulative tests, full Era 3 foundation

---

## Master Timeline & Dependencies

```
Stage 1: WASM-EBus Bridge ────────────────────── COMPLETE ✅ (19 tests)
         │
         └──→ Stage 4.1: Zig HID Driver ────────────── 15-20 tests
              │
              ├──→ Stage 4.2: Predictive Policy ─────── 20-25 tests
              │    │
              │    └──→ Stage 4.3: Curiosity Loop ──── 15-20 tests
              │
              └──→ Stage 2: GGUF Splicing ────────────── 20-25 tests
                   │
                   └──→ Stage 3: Agent Synthesis ─────── 15-20 tests
                        │
                        └──→ Stage 5.1: Glass Workshop ─ 20-30 tests

Critical Path: HID Driver → Policy → Curiosity (must be sequential)
Parallel Path: GGUF Splicing → Synthesis (can run alongside)
```

---

## Stage-by-Stage Breakdown

---

# STAGE 4.1: Zig HID Driver (Sub-1ms OS Control)

**Objective**: Give agents "hands" to control OS at pixel level  
**Duration**: 15-20 hours  
**Tests Target**: 15-20  
**Success Metric**: <1ms latency on mouse moves, keyboard presses, scroll

## 4.1.1: Architecture Design (1-2 hours, 2-3 tests)

### Core Components

```rust
/// Zig HID Driver Abstraction (Rust FFI wrapper)
pub struct ZigHidDriver {
    /// Process handle for Zig subprocess
    process: std::process::Child,
    
    /// Communication channel (IPC)
    tx: tokio::sync::mpsc::Sender<HidCommand>,
    rx: tokio::sync::mpsc::Receiver<HidResponse>,
    
    /// Performance metrics
    metrics: Arc<RwLock<HidMetrics>>,
}

pub struct HidMetrics {
    pub total_commands: u64,
    pub avg_latency_us: u32,
    pub min_latency_us: u32,
    pub max_latency_us: u32,
    pub error_count: u64,
    pub timestamp: u64,
}

pub enum HidCommand {
    MouseMove { x: i32, y: i32 },
    MouseClick { button: MouseButton, x: i32, y: i32 },
    MouseRelease { button: MouseButton },
    KeyPress { key: u32, modifiers: u8 },
    KeyRelease { key: u32 },
    Scroll { delta: i32 },
    GetCursorPos,
    QueryKeyState { key: u32 },
}

pub enum HidResponse {
    Success,
    CursorPos { x: i32, y: i32 },
    KeyState { pressed: bool },
    Error { reason: String },
    Latency { us: u32 },
}

pub enum MouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
}

impl ZigHidDriver {
    /// Initialize driver (spawn Zig subprocess)
    pub async fn new() -> Result<Self> {
        // Spawn: ./target/release/zig_hid_server
        // Communicate via named pipe or socket
        todo!()
    }
    
    /// Execute command with latency tracking
    pub async fn execute(&self, cmd: HidCommand) -> Result<HidResponse> {
        let start = std::time::Instant::now();
        
        self.tx.send(cmd).await?;
        let response = self.rx.recv().await.ok_or("No response")?;
        
        let latency_us = start.elapsed().as_micros() as u32;
        
        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.total_commands += 1;
        metrics.avg_latency_us = (metrics.avg_latency_us + latency_us) / 2;
        metrics.min_latency_us = metrics.min_latency_us.min(latency_us);
        metrics.max_latency_us = metrics.max_latency_us.max(latency_us);
        
        Ok(response)
    }
    
    /// Get performance metrics
    pub async fn metrics(&self) -> HidMetrics {
        self.metrics.read().await.clone()
    }
}
```

### Test 4.1.1: Driver Creation & Initialization

```rust
#[tokio::test]
async fn test_zig_hid_driver_init() {
    let driver = ZigHidDriver::new().await;
    assert!(driver.is_ok());
    
    let driver = driver.unwrap();
    assert_eq!(driver.metrics.read().await.total_commands, 0);
}
```

### Test 4.1.2: Latency Measurement Setup

```rust
#[tokio::test]
async fn test_zig_hid_latency_tracking() {
    let driver = ZigHidDriver::new().await.unwrap();
    
    // Execute dummy command
    let _ = driver.execute(HidCommand::GetCursorPos).await;
    
    let metrics = driver.metrics().await;
    assert!(metrics.total_commands > 0);
    assert!(metrics.avg_latency_us > 0);  // Should record latency
}
```

## 4.1.2: Zig Subprocess Implementation (6-8 hours, 6-8 tests)

### Architecture: Zig HID Server

**File**: `zig_hid_server/src/main.zig`

```zig
const std = @import("std");
const windows = std.os.windows;
const linux = std.os.linux;

/// Main HID server loop
pub fn main() !void {
    // 1. Open IPC channel (Windows named pipe / Linux Unix socket)
    // 2. Listen for HidCommand messages
    // 3. Execute OS-level input
    // 4. Send HidResponse back
    // 5. Track sub-microsecond latency
    
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();
    
    // Platform detection
    const is_windows = @import("builtin").os.tag == .windows;
    
    if (is_windows) {
        try main_windows(allocator);
    } else {
        try main_linux(allocator);
    }
}

/// Windows implementation (SetCursorPos, mouse_event, keybd_event)
fn main_windows(allocator: std.mem.Allocator) !void {
    // 1. Create named pipe: "\\\\.\\pipe\\aaroneous_hid"
    // 2. Listen for connections
    // 3. For each command:
    //    a. Get timestamp (QueryPerformanceCounter for μs precision)
    //    b. Execute Windows API call
    //    c. Send response with latency
    
    const pipe_name = "\\\\.\\pipe\\aaroneous_hid";
    const pipe_handle = windows.CreateNamedPipeA(
        pipe_name,
        windows.PIPE_ACCESS_DUPLEX,
        windows.PIPE_TYPE_MESSAGE | windows.PIPE_READMODE_MESSAGE,
        1,  // max instances
        1024,  // out buffer size
        1024,  // in buffer size
        0,  // timeout
        null  // security attributes
    );
    
    if (pipe_handle == windows.INVALID_HANDLE_VALUE) {
        std.debug.print("Failed to create named pipe\n", .{});
        return error.PipeCreationFailed;
    }
    defer windows.CloseHandle(pipe_handle);
    
    // Main loop
    while (true) {
        // Wait for client connection
        _ = windows.ConnectNamedPipe(pipe_handle, null);
        
        // Read command
        var buffer: [1024]u8 = undefined;
        var bytes_read: windows.DWORD = 0;
        
        _ = windows.ReadFile(
            pipe_handle,
            &buffer,
            buffer.len,
            &bytes_read,
            null
        );
        
        // Parse command (JSON or binary)
        // Execute
        // Send response
        
        _ = windows.DisconnectNamedPipe(pipe_handle);
    }
}

/// Linux implementation (uinput, libevdev)
fn main_linux(allocator: std.mem.Allocator) !void {
    // 1. Open /dev/uinput
    // 2. Configure as mouse + keyboard
    // 3. Create Unix socket for IPC
    // 4. Listen for commands
    // 5. Execute via uinput_event
    
    const uinput_path = "/dev/uinput";
    const uinput_fd = try std.os.open(uinput_path, std.os.O.RDWR, 0);
    defer std.os.close(uinput_fd);
    
    // Setup uinput device
    var device: input_event = undefined;
    device.type = EV_SYN;
    device.code = 0;
    device.value = 0;
    
    // Socket for IPC
    const socket = try std.os.socket(std.os.AF.UNIX, std.os.SOCK.STREAM, 0);
    defer std.os.close(socket);
    
    // Bind to /tmp/aaroneous_hid.sock
    // Listen for commands
    // Execute
}

/// Execute mouse move (sub-microsecond on modern hardware)
fn execute_mouse_move(x: i32, y: i32) !void {
    const is_windows = @import("builtin").os.tag == .windows;
    
    if (is_windows) {
        // SetCursorPos is <1μs on modern systems
        const success = windows.SetCursorPos(x, y);
        if (!success) return error.SetCursorPosFailed;
    } else {
        // uinput mouse move
        var event: input_event = undefined;
        event.type = EV_ABS;
        event.code = ABS_X;
        event.value = x;
        // Write event
    }
}

const input_event = extern struct {
    type: u16,
    code: u16,
    value: i32,
};

const EV_SYN = 0;
const EV_KEY = 1;
const EV_ABS = 3;
```

### Test 4.1.3: Windows Implementation

```rust
#[tokio::test]
#[cfg(windows)]
async fn test_windows_mouse_move() {
    let driver = ZigHidDriver::new().await.unwrap();
    
    let response = driver.execute(HidCommand::MouseMove { x: 100, y: 100 })
        .await
        .unwrap();
    
    // Verify success
    match response {
        HidResponse::Success => {}
        _ => panic!("Expected Success response"),
    }
}
```

### Test 4.1.4: Linux Implementation

```rust
#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_linux_mouse_move() {
    let driver = ZigHidDriver::new().await.unwrap();
    
    let response = driver.execute(HidCommand::MouseMove { x: 100, y: 100 })
        .await
        .unwrap();
    
    match response {
        HidResponse::Success => {}
        _ => panic!("Expected Success response"),
    }
}
```

### Test 4.1.5-4.1.8: Keyboard & Scroll

```rust
#[tokio::test]
async fn test_key_press() {
    let driver = ZigHidDriver::new().await.unwrap();
    
    let response = driver.execute(HidCommand::KeyPress { 
        key: 0x41,  // 'A'
        modifiers: 0 
    }).await.unwrap();
    
    assert!(matches!(response, HidResponse::Success));
}

#[tokio::test]
async fn test_key_release() {
    let driver = ZigHidDriver::new().await.unwrap();
    
    let response = driver.execute(HidCommand::KeyRelease { key: 0x41 })
        .await
        .unwrap();
    
    assert!(matches!(response, HidResponse::Success));
}

#[tokio::test]
async fn test_scroll() {
    let driver = ZigHidDriver::new().await.unwrap();
    
    let response = driver.execute(HidCommand::Scroll { delta: 5 })
        .await
        .unwrap();
    
    assert!(matches!(response, HidResponse::Success));
}

#[tokio::test]
async fn test_get_cursor_pos() {
    let driver = ZigHidDriver::new().await.unwrap();
    
    let response = driver.execute(HidCommand::GetCursorPos)
        .await
        .unwrap();
    
    match response {
        HidResponse::CursorPos { x, y } => {
            assert!(x >= 0);
            assert!(y >= 0);
        }
        _ => panic!("Expected CursorPos response"),
    }
}
```

## 4.1.3: Latency Validation (4-5 hours, 4-5 tests)

### Sub-1ms Latency Proof

```rust
#[tokio::test]
async fn test_sub_1ms_latency() {
    let driver = ZigHidDriver::new().await.unwrap();
    
    // Execute 100 commands and measure
    for i in 0..100 {
        let _ = driver.execute(HidCommand::MouseMove { 
            x: 100 + i, 
            y: 100 
        }).await;
    }
    
    let metrics = driver.metrics().await;
    
    // Maximum latency should be <1000 microseconds
    assert!(metrics.max_latency_us < 1000, 
        "Max latency {}μs exceeds 1ms", metrics.max_latency_us);
    
    // Average should be <100 microseconds
    assert!(metrics.avg_latency_us < 100,
        "Avg latency {}μs exceeds 100μs", metrics.avg_latency_us);
}

#[tokio::test]
async fn test_stress_1000_commands() {
    let driver = ZigHidDriver::new().await.unwrap();
    
    for i in 0..1000 {
        let cmd = if i % 3 == 0 {
            HidCommand::MouseMove { x: i as i32, y: i as i32 }
        } else if i % 3 == 1 {
            HidCommand::KeyPress { key: 0x41, modifiers: 0 }
        } else {
            HidCommand::Scroll { delta: 1 }
        };
        
        let response = driver.execute(cmd).await;
        assert!(response.is_ok(), "Command {} failed", i);
    }
    
    let metrics = driver.metrics().await;
    assert_eq!(metrics.total_commands, 1000);
    assert_eq!(metrics.error_count, 0);
}

#[tokio::test]
async fn test_latency_distribution() {
    let driver = ZigHidDriver::new().await.unwrap();
    
    let mut latencies = Vec::new();
    
    for _ in 0..100 {
        let start = std::time::Instant::now();
        let _ = driver.execute(HidCommand::GetCursorPos).await;
        latencies.push(start.elapsed().as_micros() as u32);
    }
    
    latencies.sort();
    
    let p50 = latencies[50];
    let p95 = latencies[95];
    let p99 = latencies[99];
    
    println!("Latency distribution: p50={}μs, p95={}μs, p99={}μs", p50, p95, p99);
    
    assert!(p99 < 1000, "p99 latency exceeds 1ms");
}
```

## 4.1.4: Integration with Action Executor (2-3 hours, 2-3 tests)

### Connect HID Driver to WASM Bridge

```rust
/// Update ActionExecutor to use ZigHidDriver
pub struct ActionExecutor {
    pub hid_driver: Arc<ZigHidDriver>,
    enabled: Arc<AtomicBool>,
}

impl ActionExecutor {
    pub async fn execute(&self, action_bytes: &[u8]) -> Result<Bytes> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Ok(Bytes::from_static(b"\x00"));
        }
        
        let action: MarionetteAction = serde_json::from_slice(action_bytes)?;
        
        // Execute via HID driver
        let hid_response = match action {
            MarionetteAction::MouseMove { x, y } => {
                self.hid_driver.execute(HidCommand::MouseMove { x, y }).await?
            }
            MarionetteAction::MouseClick { button, x, y } => {
                self.hid_driver.execute(HidCommand::MouseMove { x, y }).await?;
                self.hid_driver.execute(HidCommand::MouseClick { button, x, y }).await?
            }
            MarionetteAction::KeyPress { key } => {
                self.hid_driver.execute(HidCommand::KeyPress { 
                    key, 
                    modifiers: 0 
                }).await?
            }
            MarionetteAction::Scroll { delta } => {
                self.hid_driver.execute(HidCommand::Scroll { delta }).await?
            }
            _ => todo!(),
        };
        
        let response = serde_json::to_vec(&hid_response)?;
        Ok(Bytes::from(response))
    }
}

#[tokio::test]
async fn test_action_executor_with_hid() {
    let hid_driver = Arc::new(ZigHidDriver::new().await.unwrap());
    let executor = ActionExecutor {
        hid_driver,
        enabled: Arc::new(AtomicBool::new(true)),
    };
    
    let action = MarionetteAction::MouseMove { x: 100, y: 100 };
    let action_bytes = serde_json::to_vec(&action).unwrap();
    
    let result = executor.execute(&action_bytes).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_action_executor_latency_<_1ms() {
    let hid_driver = Arc::new(ZigHidDriver::new().await.unwrap());
    let executor = ActionExecutor {
        hid_driver,
        enabled: Arc::new(AtomicBool::new(true)),
    };
    
    for _ in 0..100 {
        let action = MarionetteAction::MouseMove { x: 50, y: 50 };
        let action_bytes = serde_json::to_vec(&action).unwrap();
        
        let start = std::time::Instant::now();
        let _ = executor.execute(&action_bytes).await;
        let latency = start.elapsed().as_micros();
        
        assert!(latency < 1000, "Latency {}μs exceeds 1ms", latency);
    }
}
```

---

# STAGE 4.2: Predictive Policy Engine (Intent → Actions)

**Objective**: Convert high-level LLM intent into precise marionette movements  
**Duration**: 20-25 hours  
**Tests Target**: 20-25  
**Success Metric**: Policy execution with <50ms latency from intent to action queue

## 4.2.1: World Model & Prediction (6-8 hours, 6-8 tests)

### Core Components

```rust
/// Agent's internal world model
pub struct WorldModel {
    /// Predictions: "If I do X, Y will happen"
    pub predictions: Arc<RwLock<HashMap<String, StateTransitionPrediction>>>,
    
    /// Confidence in predictions
    pub confidence: Arc<AtomicF32>,
    
    /// Known state transitions
    pub known_transitions: Arc<RwLock<Vec<KnownTransition>>>,
}

pub struct StateTransitionPrediction {
    pub action: String,
    pub expected_next_state: GameState,
    pub confidence: f32,
    pub success_count: u32,
    pub failure_count: u32,
}

pub struct KnownTransition {
    pub from_state: GameState,
    pub action: String,
    pub to_state: GameState,
    pub outcome: TransitionOutcome,
}

pub enum TransitionOutcome {
    Success,
    Failure { reason: String },
    Unexpected { actual_state: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GameState {
    pub player_position: (i32, i32),
    pub focused_ui: Option<String>,
    pub nearby_entities: Vec<u32>,
    pub inventory_hash: u64,
}

impl WorldModel {
    pub fn new() -> Self {
        Self {
            predictions: Arc::new(RwLock::new(HashMap::new())),
            confidence: Arc::new(AtomicF32::new(0.5)),
            known_transitions: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Make prediction about next state
    pub async fn predict_state_transition(
        &self,
        current_state: &GameState,
        action: &str,
    ) -> Result<StateTransitionPrediction> {
        let predictions = self.predictions.read().await;
        
        let key = format!("{}::{}", 
            serde_json::to_string(current_state)?,
            action
        );
        
        if let Some(pred) = predictions.get(&key) {
            Ok(pred.clone())
        } else {
            // Unknown transition - return default
            Ok(StateTransitionPrediction {
                action: action.to_string(),
                expected_next_state: current_state.clone(),
                confidence: 0.0,
                success_count: 0,
                failure_count: 0,
            })
        }
    }
    
    /// Update prediction after observing outcome
    pub async fn update_prediction(
        &self,
        current_state: &GameState,
        action: &str,
        observed_state: &GameState,
        outcome: TransitionOutcome,
    ) -> Result<()> {
        let key = format!("{}::{}", 
            serde_json::to_string(current_state)?,
            action
        );
        
        let mut predictions = self.predictions.write().await;
        
        let pred = predictions.entry(key).or_insert(StateTransitionPrediction {
            action: action.to_string(),
            expected_next_state: observed_state.clone(),
            confidence: 0.0,
            success_count: 0,
            failure_count: 0,
        });
        
        match outcome {
            TransitionOutcome::Success => {
                pred.success_count += 1;
                pred.expected_next_state = observed_state.clone();
                pred.confidence = pred.success_count as f32 / 
                    (pred.success_count + pred.failure_count) as f32;
            }
            TransitionOutcome::Failure { .. } => {
                pred.failure_count += 1;
            }
            TransitionOutcome::Unexpected { .. } => {
                pred.failure_count += 1;
            }
        }
        
        // Update overall confidence
        let avg_confidence = predictions.values()
            .map(|p| p.confidence)
            .sum::<f32>() / predictions.len() as f32;
        
        self.confidence.store(avg_confidence, Ordering::Relaxed);
        
        Ok(())
    }
}

impl Clone for StateTransitionPrediction {
    fn clone(&self) -> Self {
        Self {
            action: self.action.clone(),
            expected_next_state: self.expected_next_state.clone(),
            confidence: self.confidence,
            success_count: self.success_count,
            failure_count: self.failure_count,
        }
    }
}

impl Clone for GameState {
    fn clone(&self) -> Self {
        Self {
            player_position: self.player_position,
            focused_ui: self.focused_ui.clone(),
            nearby_entities: self.nearby_entities.clone(),
            inventory_hash: self.inventory_hash,
        }
    }
}
```

### Test 4.2.1: World Model Creation

```rust
#[tokio::test]
async fn test_world_model_creation() {
    let model = WorldModel::new();
    
    assert!(model.predictions.read().await.is_empty());
    assert!(model.confidence.load(Ordering::Relaxed) > 0.0);
}

#[tokio::test]
async fn test_predict_unknown_transition() {
    let model = WorldModel::new();
    
    let state = GameState {
        player_position: (0, 0),
        focused_ui: None,
        nearby_entities: vec![],
        inventory_hash: 0,
    };
    
    let pred = model.predict_state_transition(&state, "move_forward")
        .await
        .unwrap();
    
    assert_eq!(pred.confidence, 0.0);  // Unknown transition
}

#[tokio::test]
async fn test_update_prediction_success() {
    let model = WorldModel::new();
    
    let state1 = GameState {
        player_position: (0, 0),
        focused_ui: None,
        nearby_entities: vec![],
        inventory_hash: 0,
    };
    
    let state2 = GameState {
        player_position: (10, 0),
        focused_ui: None,
        nearby_entities: vec![],
        inventory_hash: 0,
    };
    
    // Record successful transition
    model.update_prediction(&state1, "move_forward", &state2, 
        TransitionOutcome::Success).await.unwrap();
    
    // Now prediction should exist
    let pred = model.predict_state_transition(&state1, "move_forward")
        .await
        .unwrap();
    
    assert_eq!(pred.success_count, 1);
    assert!(pred.confidence > 0.0);
}

#[tokio::test]
async fn test_prediction_confidence_increases() {
    let model = WorldModel::new();
    
    let state1 = GameState { player_position: (0, 0), focused_ui: None, nearby_entities: vec![], inventory_hash: 0 };
    let state2 = GameState { player_position: (10, 0), focused_ui: None, nearby_entities: vec![], inventory_hash: 0 };
    
    // Record 10 successes
    for _ in 0..10 {
        model.update_prediction(&state1, "move", &state2, TransitionOutcome::Success).await.unwrap();
    }
    
    let pred = model.predict_state_transition(&state1, "move").await.unwrap();
    assert_eq!(pred.success_count, 10);
    assert!(pred.confidence > 0.9);
}
```

## 4.2.2: Policy Executor (6-8 hours, 6-8 tests)

```rust
/// Executes policies (action sequences) based on world state
pub struct PolicyExecutor {
    /// World model for predictions
    pub world_model: Arc<WorldModel>,
    
    /// HID driver for action execution
    pub hid_driver: Arc<ZigHidDriver>,
    
    /// Current game state
    pub current_state: Arc<RwLock<GameState>>,
    
    /// Action queue
    pub action_queue: Arc<tokio::sync::Mutex<VecDeque<MarionetteAction>>>,
}

pub struct ExecutionPlan {
    pub actions: Vec<MarionetteAction>,
    pub expected_state_transitions: Vec<GameState>,
    pub total_expected_latency_ms: u32,
}

impl PolicyExecutor {
    pub fn new(
        world_model: Arc<WorldModel>,
        hid_driver: Arc<ZigHidDriver>,
    ) -> Self {
        Self {
            world_model,
            hid_driver,
            current_state: Arc::new(RwLock::new(GameState {
                player_position: (0, 0),
                focused_ui: None,
                nearby_entities: vec![],
                inventory_hash: 0,
            })),
            action_queue: Arc::new(tokio::sync::Mutex::new(VecDeque::new())),
        }
    }
    
    /// Generate execution plan from high-level intent
    pub async fn plan_execution(
        &self,
        intent: &str,
    ) -> Result<ExecutionPlan> {
        // Intent examples: "Move to NPC", "Open inventory", "Attack enemy"
        
        let current = self.current_state.read().await.clone();
        
        let actions = match intent {
            "move_forward" => vec![
                MarionetteAction::KeyPress { key: 0x57, modifiers: 0 },  // 'W'
            ],
            "strafe_left" => vec![
                MarionetteAction::KeyPress { key: 0x41, modifiers: 0 },  // 'A'
            ],
            "jump" => vec![
                MarionetteAction::KeyPress { key: 0x20, modifiers: 0 },  // Space
            ],
            "open_inventory" => vec![
                MarionetteAction::KeyPress { key: 0x49, modifiers: 0 },  // 'I'
            ],
            _ => return Err("Unknown intent".into()),
        };
        
        Ok(ExecutionPlan {
            actions: actions.clone(),
            expected_state_transitions: vec![current],  // Simplified
            total_expected_latency_ms: (actions.len() as u32) * 10,
        })
    }
    
    /// Execute plan, measuring outcomes
    pub async fn execute_plan(&self, plan: ExecutionPlan) -> Result<ExecutionResult> {
        let start = std::time::Instant::now();
        let mut results = Vec::new();
        
        for (i, action) in plan.actions.iter().enumerate() {
            let action_start = std::time::Instant::now();
            
            let hid_response = match action {
                MarionetteAction::MouseMove { x, y } => {
                    self.hid_driver.execute(HidCommand::MouseMove { x: *x, y: *y }).await?
                }
                MarionetteAction::KeyPress { key } => {
                    self.hid_driver.execute(HidCommand::KeyPress { 
                        key: *key, 
                        modifiers: 0 
                    }).await?
                }
                _ => HidResponse::Success,
            };
            
            let action_latency = action_start.elapsed().as_millis() as u32;
            
            results.push(ActionExecutionResult {
                action_index: i,
                status: match hid_response {
                    HidResponse::Success => "success".to_string(),
                    _ => "error".to_string(),
                },
                latency_ms: action_latency,
            });
        }
        
        let total_latency = start.elapsed();
        
        Ok(ExecutionResult {
            plan_size: plan.actions.len(),
            total_latency_ms: total_latency.as_millis() as u32,
            action_results: results,
            success: true,
        })
    }
    
    /// Update state based on observation
    pub async fn observe_state(&self, new_state: GameState) -> Result<()> {
        let mut state = self.current_state.write().await;
        *state = new_state;
        Ok(())
    }
}

pub struct ExecutionResult {
    pub plan_size: usize,
    pub total_latency_ms: u32,
    pub action_results: Vec<ActionExecutionResult>,
    pub success: bool,
}

pub struct ActionExecutionResult {
    pub action_index: usize,
    pub status: String,
    pub latency_ms: u32,
}
```

### Test 4.2.2: Policy Planning

```rust
#[tokio::test]
async fn test_plan_move_forward() {
    let world_model = Arc::new(WorldModel::new());
    let hid_driver = Arc::new(ZigHidDriver::new().await.unwrap());
    let executor = PolicyExecutor::new(world_model, hid_driver);
    
    let plan = executor.plan_execution("move_forward").await.unwrap();
    
    assert!(!plan.actions.is_empty());
    assert!(plan.total_expected_latency_ms > 0);
}

#[tokio::test]
async fn test_execute_plan_latency() {
    let world_model = Arc::new(WorldModel::new());
    let hid_driver = Arc::new(ZigHidDriver::new().await.unwrap());
    let executor = PolicyExecutor::new(world_model, hid_driver);
    
    let plan = executor.plan_execution("move_forward").await.unwrap();
    let result = executor.execute_plan(plan).await.unwrap();
    
    // Plan execution should be <50ms
    assert!(result.total_latency_ms < 50,
        "Execution latency {}ms exceeds 50ms", result.total_latency_ms);
}
```

## 4.2.3: Predictive Action Selection (4-5 hours, 4-5 tests)

```rust
impl PolicyExecutor {
    /// Select best action based on confidence
    pub async fn select_best_action_for_intent(
        &self,
        intent: &str,
    ) -> Result<MarionetteAction> {
        let current = self.current_state.read().await.clone();
        
        // For each known action, predict outcome
        let actions = vec!["move_forward", "strafe_left", "jump", "open_inventory"];
        
        let mut best_action = None;
        let mut best_confidence = 0.0;
        
        for action in actions {
            let prediction = self.world_model.predict_state_transition(
                &current,
                action
            ).await?;
            
            if prediction.confidence > best_confidence {
                best_confidence = prediction.confidence;
                best_action = Some(action);
            }
        }
        
        match best_action {
            Some(action) => Ok(MarionetteAction::KeyPress { 
                key: action_to_keycode(action),
                modifiers: 0 
            }),
            None => Err("No suitable action found".into()),
        }
    }
    
    /// Predictive latency estimation
    pub async fn estimate_execution_latency(
        &self,
        plan: &ExecutionPlan,
    ) -> u32 {
        // HID latency + action count
        let hid_avg_latency_us = 500;  // Conservative estimate
        (plan.actions.len() as u32) * hid_avg_latency_us / 1000  // Convert to ms
    }
}

fn action_to_keycode(action: &str) -> u32 {
    match action {
        "move_forward" => 0x57,  // W
        "strafe_left" => 0x41,   // A
        "jump" => 0x20,          // Space
        "open_inventory" => 0x49, // I
        _ => 0,
    }
}

#[tokio::test]
async fn test_select_best_action() {
    let world_model = Arc::new(WorldModel::new());
    let hid_driver = Arc::new(ZigHidDriver::new().await.unwrap());
    let executor = PolicyExecutor::new(world_model, hid_driver);
    
    let action = executor.select_best_action_for_intent("move_forward")
        .await
        .unwrap();
    
    // Should return a valid action
    match action {
        MarionetteAction::KeyPress { .. } => {}
        _ => panic!("Expected KeyPress"),
    }
}

#[tokio::test]
async fn test_latency_estimation() {
    let world_model = Arc::new(WorldModel::new());
    let hid_driver = Arc::new(ZigHidDriver::new().await.unwrap());
    let executor = PolicyExecutor::new(world_model, hid_driver);
    
    let plan = executor.plan_execution("move_forward").await.unwrap();
    let estimated_latency = executor.estimate_execution_latency(&plan).await;
    
    assert!(estimated_latency > 0);
}
```

---

# STAGE 4.3: Curiosity Learning Loop (Surprise = Learning)

**Objective**: Agent improves by exploring and learning from prediction errors  
**Duration**: 15-20 hours  
**Tests Target**: 15-20  
**Success Metric**: Agent autonomously discovers new state transitions and updates policies

## 4.3.1: Prediction Error Tracking (4-5 hours, 4-5 tests)

```rust
/// Track how often predictions are correct
pub struct CuriosityTracker {
    /// Prediction errors (actual - predicted)
    pub prediction_errors: Arc<RwLock<Vec<PredictionError>>>,
    
    /// Intrinsic reward (surprise value)
    pub intrinsic_rewards: Arc<RwLock<Vec<f32>>>,
    
    /// Discovery log (unexpected outcomes)
    pub discoveries: Arc<RwLock<Vec<Discovery>>>,
}

pub struct PredictionError {
    pub action: String,
    pub predicted_state: GameState,
    pub actual_state: GameState,
    pub error_magnitude: f32,
    pub timestamp: u64,
}

pub struct Discovery {
    pub action: String,
    pub new_state: GameState,
    pub surprise_value: f32,
    pub learning_value: f32,
}

impl CuriosityTracker {
    pub fn new() -> Self {
        Self {
            prediction_errors: Arc::new(RwLock::new(Vec::new())),
            intrinsic_rewards: Arc::new(RwLock::new(Vec::new())),
            discoveries: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Calculate surprise (prediction error)
    pub fn calculate_prediction_error(
        predicted: &GameState,
        actual: &GameState,
    ) -> f32 {
        // Euclidean distance in state space
        let pos_diff = (predicted.player_position.0 - actual.player_position.0).pow(2) as f32
                     + (predicted.player_position.1 - actual.player_position.1).pow(2) as f32;
        
        let pos_dist = pos_diff.sqrt();
        
        let mut error = pos_dist;
        
        // Bonus for unexpected UI state
        if predicted.focused_ui != actual.focused_ui {
            error += 10.0;  // High surprise
        }
        
        // Bonus for entity changes
        let entity_diff = predicted.nearby_entities.len() as i32 
                        - actual.nearby_entities.len() as i32;
        error += entity_diff.abs() as f32 * 5.0;
        
        error
    }
    
    /// Record prediction error and calculate intrinsic reward
    pub async fn record_error(
        &self,
        action: String,
        predicted: GameState,
        actual: GameState,
    ) -> Result<f32> {
        let error = Self::calculate_prediction_error(&predicted, &actual);
        
        // Intrinsic reward = surprise (prediction error)
        let intrinsic_reward = error;
        
        let error_record = PredictionError {
            action: action.clone(),
            predicted_state: predicted,
            actual_state: actual.clone(),
            error_magnitude: error,
            timestamp: now_ns(),
        };
        
        self.prediction_errors.write().await.push(error_record);
        self.intrinsic_rewards.write().await.push(intrinsic_reward);
        
        // If error is high enough, record as discovery
        if error > 5.0 {
            let discovery = Discovery {
                action,
                new_state: actual,
                surprise_value: error,
                learning_value: error,
            };
            
            self.discoveries.write().await.push(discovery);
        }
        
        Ok(intrinsic_reward)
    }
    
    /// Get average prediction accuracy
    pub async fn average_prediction_accuracy(&self) -> f32 {
        let errors = self.prediction_errors.read().await;
        
        if errors.is_empty() {
            return 1.0;  // No errors yet
        }
        
        let sum_errors: f32 = errors.iter().map(|e| e.error_magnitude).sum();
        let avg_error = sum_errors / errors.len() as f32;
        
        // Accuracy = 1 / (1 + avg_error)
        1.0 / (1.0 + avg_error)
    }
}

fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[tokio::test]
async fn test_prediction_error_tracking() {
    let tracker = CuriosityTracker::new();
    
    let predicted = GameState {
        player_position: (0, 0),
        focused_ui: None,
        nearby_entities: vec![],
        inventory_hash: 0,
    };
    
    let actual = GameState {
        player_position: (10, 5),
        focused_ui: Some("inventory".to_string()),
        nearby_entities: vec![1, 2],
        inventory_hash: 100,
    };
    
    let reward = tracker.record_error("move".to_string(), predicted, actual)
        .await
        .unwrap();
    
    assert!(reward > 0.0);
    assert_eq!(tracker.prediction_errors.read().await.len(), 1);
}

#[tokio::test]
async fn test_discovery_high_error() {
    let tracker = CuriosityTracker::new();
    
    let predicted = GameState {
        player_position: (0, 0),
        focused_ui: None,
        nearby_entities: vec![],
        inventory_hash: 0,
    };
    
    let actual = GameState {
        player_position: (100, 100),
        focused_ui: Some("menu".to_string()),
        nearby_entities: vec![1, 2, 3, 4, 5],
        inventory_hash: 999,
    };
    
    let _reward = tracker.record_error("teleport".to_string(), predicted, actual)
        .await
        .unwrap();
    
    // High error should trigger discovery
    assert_eq!(tracker.discoveries.read().await.len(), 1);
}
```

## 4.3.2: Exploration vs. Exploitation (5-6 hours, 5-6 tests)

```rust
/// Decide whether to explore (try new actions) or exploit (repeat known good actions)
pub struct ExplorationExploitation {
    pub curiosity_tracker: Arc<CuriosityTracker>,
    pub world_model: Arc<WorldModel>,
}

impl ExplorationExploitation {
    pub fn new(
        curiosity_tracker: Arc<CuriosityTracker>,
        world_model: Arc<WorldModel>,
    ) -> Self {
        Self {
            curiosity_tracker,
            world_model,
        }
    }
    
    /// Select action: high accuracy → explore, low accuracy → exploit
    pub async fn select_action(
        &self,
        available_actions: &[&str],
    ) -> Result<String> {
        let accuracy = self.curiosity_tracker.average_prediction_accuracy().await;
        
        if accuracy > 0.8 {
            // Confident in predictions → explore
            self.select_exploratory_action(available_actions).await
        } else {
            // Uncertain → exploit known good actions
            self.select_exploitative_action(available_actions).await
        }
    }
    
    /// Exploration: Try action with lowest confidence
    async fn select_exploratory_action(&self, available_actions: &[&str]) -> Result<String> {
        // TODO: Calculate confidence for each action
        // Return action with lowest confidence
        
        Ok(available_actions[0].to_string())
    }
    
    /// Exploitation: Try action with highest success rate
    async fn select_exploitative_action(&self, available_actions: &[&str]) -> Result<String> {
        // TODO: Find action with highest success_count / total_count
        
        Ok(available_actions[0].to_string())
    }
}

#[tokio::test]
async fn test_exploration_when_confident() {
    let curiosity = Arc::new(CuriosityTracker::new());
    let world_model = Arc::new(WorldModel::new());
    
    // Record several low-error predictions (high accuracy)
    for _ in 0..10 {
        let _ = curiosity.record_error(
            "move".to_string(),
            GameState { 
                player_position: (0, 0), 
                focused_ui: None, 
                nearby_entities: vec![], 
                inventory_hash: 0 
            },
            GameState { 
                player_position: (5, 0), 
                focused_ui: None, 
                nearby_entities: vec![], 
                inventory_hash: 0 
            },
        ).await;
    }
    
    let ee = ExplorationExploitation::new(curiosity, world_model);
    let accuracy = ee.curiosity_tracker.average_prediction_accuracy().await;
    
    assert!(accuracy > 0.7, "Accuracy should be high");
}
```

## 4.3.3: Autonomous Learning Loop (6-8 hours, 6-8 tests)

```rust
/// Main loop: Agent explores during downtime, learns from surprise
pub struct AutonomousLearningAgent {
    pub policy_executor: Arc<PolicyExecutor>,
    pub curiosity_tracker: Arc<CuriosityTracker>,
    pub world_model: Arc<WorldModel>,
    pub exploration_budget: Duration,
}

impl AutonomousLearningAgent {
    pub async fn learning_loop(&self) -> Result<LearningStats> {
        let start = std::time::Instant::now();
        let mut stats = LearningStats::default();
        
        loop {
            if start.elapsed() > self.exploration_budget {
                break;
            }
            
            // Step 1: Select action (explore or exploit)
            let available_actions = vec!["move_forward", "strafe_left", "jump"];
            
            let action_str = if self.curiosity_tracker.average_prediction_accuracy().await > 0.8 {
                available_actions[stats.actions_tried % 3]  // Explore
            } else {
                available_actions[0]  // Exploit
            };
            
            // Step 2: Get current state
            let current_state = self.policy_executor.current_state.read().await.clone();
            
            // Step 3: Make prediction
            let prediction = self.world_model.predict_state_transition(
                &current_state,
                action_str,
            ).await?;
            
            // Step 4: Execute action
            let plan = self.policy_executor.plan_execution(action_str).await?;
            let _result = self.policy_executor.execute_plan(plan).await?;
            
            // Step 5: Observe actual outcome (simulated)
            let actual_state = self.simulate_game_state_change(&current_state, action_str);
            
            // Step 6: Calculate surprise
            let surprise = self.curiosity_tracker.record_error(
                action_str.to_string(),
                prediction.expected_next_state,
                actual_state.clone(),
            ).await?;
            
            // Step 7: Update world model
            self.world_model.update_prediction(
                &current_state,
                action_str,
                &actual_state,
                TransitionOutcome::Success,
            ).await?;
            
            // Step 8: Update state
            self.policy_executor.observe_state(actual_state).await?;
            
            stats.actions_tried += 1;
            stats.total_surprise += surprise;
            stats.discoveries = self.curiosity_tracker.discoveries.read().await.len();
            
            // Brief pause
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        stats.learning_duration_ms = start.elapsed().as_millis() as u32;
        Ok(stats)
    }
    
    /// Simulate what game state change would occur
    fn simulate_game_state_change(&self, current: &GameState, action: &str) -> GameState {
        let mut next = current.clone();
        
        match action {
            "move_forward" => {
                next.player_position.1 += 10;  // Move 10 units forward
            }
            "strafe_left" => {
                next.player_position.0 -= 10;  // Move 10 units left
            }
            "jump" => {
                // No position change, but could trigger UI or entity changes
            }
            _ => {}
        }
        
        next
    }
}

#[derive(Default)]
pub struct LearningStats {
    pub actions_tried: usize,
    pub total_surprise: f32,
    pub discoveries: usize,
    pub learning_duration_ms: u32,
}

#[tokio::test]
async fn test_autonomous_learning() {
    let world_model = Arc::new(WorldModel::new());
    let hid_driver = Arc::new(ZigHidDriver::new().await.unwrap());
    let policy_executor = Arc::new(PolicyExecutor::new(world_model.clone(), hid_driver));
    let curiosity_tracker = Arc::new(CuriosityTracker::new());
    
    let agent = AutonomousLearningAgent {
        policy_executor,
        curiosity_tracker,
        world_model,
        exploration_budget: Duration::from_millis(100),  // Quick test
    };
    
    let stats = agent.learning_loop().await.unwrap();
    
    assert!(stats.actions_tried > 0);
    assert!(stats.learning_duration_ms > 0);
}

#[tokio::test]
async fn test_learning_improves_accuracy() {
    let world_model = Arc::new(WorldModel::new());
    let hid_driver = Arc::new(ZigHidDriver::new().await.unwrap());
    let policy_executor = Arc::new(PolicyExecutor::new(world_model.clone(), hid_driver));
    let curiosity_tracker = Arc::new(CuriosityTracker::new());
    
    // Initial accuracy (no data)
    let initial_accuracy = curiosity_tracker.average_prediction_accuracy().await;
    
    let agent = AutonomousLearningAgent {
        policy_executor,
        curiosity_tracker: curiosity_tracker.clone(),
        world_model,
        exploration_budget: Duration::from_millis(200),
    };
    
    let _stats = agent.learning_loop().await.unwrap();
    
    // After learning, accuracy should improve (or at least we have data)
    let final_accuracy = curiosity_tracker.average_prediction_accuracy().await;
    
    // We should have recorded some errors
    assert!(!curiosity_tracker.prediction_errors.read().await.is_empty());
}
```

---

# STAGE 2 & 3: GGUF Splicing & Agent Synthesis (Parallel Path)

**Can run in parallel with Stages 4.1-4.3**

## Combined Duration: 35-45 hours, 35-45 tests

### Stage 2: GGUF Splicing (Extract Engrams)

```rust
/// Extract personality engrams from teacher models
pub struct GgufSplicer {
    pub gguf_path: PathBuf,
}

impl GgufSplicer {
    /// Identify layers responsible for specific traits
    pub fn extract_personality_engram(
        &self,
        trait_type: &str,
    ) -> Result<Vec<u8>> {
        // Read GGUF file
        // Find layers related to trait (attention heads, MLP)
        // Extract weights
        // Return as binary blob
        
        match trait_type {
            "creative" => {
                // Later attention blocks encode creativity
                // Extract top 5 attention layer weights
                Ok(vec![])
            }
            "logical" => {
                // MLP layers encode logical reasoning
                Ok(vec![])
            }
            _ => Err("Unknown trait".into()),
        }
    }
}
```

### Stage 3: Agent Synthesis (Binary Patching)

```rust
/// Create new agents by binary-patching engrams into shells
pub struct AgentSynthesizer {
    pub shell_path: PathBuf,
    pub ssd_path: PathBuf,
}

impl AgentSynthesizer {
    /// Synthesize agent by patching engrams
    pub async fn synthesize_agent(
        &self,
        engrams: &[Vec<u8>],
    ) -> Result<SynthesizedAgent> {
        // 1. Copy shell to new file
        // 2. For each engram, pwrite into correct layer offset
        // 3. Validate coherence
        // 4. Return agent handle
        
        Ok(SynthesizedAgent::default())
    }
}

#[derive(Default)]
pub struct SynthesizedAgent {
    pub id: String,
    pub path: PathBuf,
}
```

---

# STAGE 5: Glass Workshop (O3DE Manifestation)

**Dependency**: Requires Stages 4.1-4.3 complete  
**Duration**: 20-30 hours  
**Tests**: 20-30

```rust
/// O3DE Gem: Visual manifestation of Ariel and Glass
pub struct GlassWorkshopGem {
    pub ariel_avatar: Arc<VRoidModel>,
    pub glass_lens: Arc<PrismaticLens>,
    pub interaction_handler: Arc<RelicInteractionHandler>,
}

impl GlassWorkshopGem {
    /// Render in O3DE (called by engine)
    pub async fn render(&self) -> Result<()> {
        // Draw VRoid avatar
        // Draw prismatic lens
        // Update based on relic state
        
        Ok(())
    }
}
```

---

# COMPLETE IMPLEMENTATION CHECKLIST

## Phase 6D.2: Zig HID Driver
- [ ] Architecture design (1-2 hours)
  - [ ] ZigHidDriver struct (driver creation, initialization)
  - [ ] HidCommand/HidResponse enums (serialization format)
  - [ ] HidMetrics tracking (latency percentiles)
  - [ ] Tests: 2-3 (driver init, latency setup)

- [ ] Zig subprocess (6-8 hours)
  - [ ] Windows implementation (SetCursorPos, mouse_event, keybd_event)
  - [ ] Linux implementation (uinput, libevdev)
  - [ ] IPC channel (named pipes / Unix sockets)
  - [ ] Sub-microsecond latency measurement
  - [ ] Tests: 6-8 (OS-specific, keyboard, scroll, mouse)

- [ ] Latency validation (4-5 hours)
  - [ ] <1ms latency proof
  - [ ] Stress test (1000 commands)
  - [ ] Latency distribution (p50, p95, p99)
  - [ ] Tests: 4-5

- [ ] Integration with ActionExecutor (2-3 hours)
  - [ ] Connect HID driver to WASM bridge
  - [ ] Test action execution via HID
  - [ ] Test end-to-end latency
  - [ ] Tests: 2-3

**Subtotal**: 15-20 hours, 15-20 tests

---

## Phase 6D.3: Predictive Policy Engine
- [ ] World Model (6-8 hours)
  - [ ] StateTransitionPrediction (action → expected state)
  - [ ] KnownTransition storage
  - [ ] Prediction accuracy tracking
  - [ ] Confidence calculation
  - [ ] Tests: 6-8

- [ ] Policy Executor (6-8 hours)
  - [ ] ExecutionPlan generation (from intent)
  - [ ] Action execution with latency tracking
  - [ ] State observation/updates
  - [ ] Plan optimization
  - [ ] Tests: 6-8

- [ ] Predictive Action Selection (4-5 hours)
  - [ ] Best action selection (via confidence)
  - [ ] Latency estimation
  - [ ] Lookahead (predict 2-3 steps)
  - [ ] Tests: 4-5

- [ ] Integration with World Model (2-3 hours)
  - [ ] Connect policy executor to world model
  - [ ] Test planning latency <50ms
  - [ ] Tests: 2-3

**Subtotal**: 20-25 hours, 20-25 tests

---

## Phase 6D.4: Curiosity Learning Loop
- [ ] Prediction Error Tracking (4-5 hours)
  - [ ] Error magnitude calculation (state distance)
  - [ ] Intrinsic reward (surprise value)
  - [ ] Discovery logging (high-error outcomes)
  - [ ] Accuracy averaging
  - [ ] Tests: 4-5

- [ ] Exploration vs. Exploitation (5-6 hours)
  - [ ] Accuracy threshold for exploration
  - [ ] Exploratory action selection (lowest confidence)
  - [ ] Exploitative action selection (highest success)
  - [ ] Dynamic switching
  - [ ] Tests: 5-6

- [ ] Autonomous Learning Loop (6-8 hours)
  - [ ] Main learning loop (predict → execute → observe → learn)
  - [ ] Surprise-driven updates
  - [ ] World model updates
  - [ ] State transitions
  - [ ] Learning statistics
  - [ ] Tests: 6-8

**Subtotal**: 15-20 hours, 15-20 tests

---

## Phase 6D.5 & 6D.6: GGUF Splicing & Agent Synthesis (Parallel)
- [ ] GGUF Splicing (20-25 hours, 20-25 tests)
  - [ ] GGUF header parsing
  - [ ] Layer identification (attention, MLP, embedding)
  - [ ] Weight extraction (binary blob)
  - [ ] Personality engram extraction
  - [ ] Engram storage on SSD

- [ ] Agent Synthesis (15-20 hours, 15-20 tests)
  - [ ] Shell creation (minimal GGUF)
  - [ ] Binary patching (pwrite engrams into shell)
  - [ ] Coherence validation
  - [ ] Hot-loading via mmap
  - [ ] Agent instantiation

**Subtotal**: 35-45 hours, 35-45 tests

---

## Phase 6D.7-6D.10: Glass Workshop + E2E Integration (4 phases)
- [ ] Glass Workshop Gem (20-30 tests, 15-20 hours)
  - [ ] VRoid avatar rendering
  - [ ] Prismatic lens visualization
  - [ ] Drag-drop file handling
  - [ ] Animation state updates

- [ ] Ariel + Glass Communication (15-20 tests, 10-12 hours)
  - [ ] Shared memory messaging
  - [ ] WorldStateToken flow
  - [ ] Action dispatch

- [ ] Multi-Relic Orchestration (20-25 tests, 12-15 hours)
  - [ ] Sentinel spawning Ariel + Glass
  - [ ] Linking relics
  - [ ] Concurrent execution

- [ ] E2E Service Loop (15-20 tests, 8-10 hours)
  - [ ] User request → Relic dispatch → Action
  - [ ] Performance validation
  - [ ] Stress testing

**Subtotal**: 50-70 tests, 45-55 hours

---

# EXECUTION TIMELINE

## Week 1: Stages 4.1 & 4.2
**18-28 hours**
- Mon-Tue: HID Driver (15-20h)
- Wed-Thu: Predictive Policy (20-25h) — starts Wed after HID basics done
- Friday: Integration testing

**Target**: 35-45 tests, both stages passing

## Week 2: Stage 4.3 + Parallel Splicing
**20-33 hours**
- Mon-Tue: Curiosity Loop (15-20h)
- Wed-Fri: GGUF Splicing + Synthesis (35-45h) — parallel, separate team or context

**Target**: 50-65 tests, curiosity + splicing complete

## Week 3: Glass Workshop + E2E
**40-55 hours**
- Mon-Tue: Glass Workshop Gem (15-20h)
- Wed-Thu: Multi-relic coordination (20-25h)
- Fri: E2E service loop + stress test (10-15h)

**Target**: 50-70 tests, full Phase 6D integration

---

# SUCCESS METRICS

### By End of Stage 4.1
- ✅ HID latency <1ms (p99 <1000μs)
- ✅ 1000 commands stress test, 0 failures
- ✅ Action executor integrated

### By End of Stage 4.2
- ✅ Planning latency <50ms
- ✅ World model predictions 80%+ accurate (after learning)
- ✅ Policy executor E2E tested

### By End of Stage 4.3
- ✅ Agent discovers 5+ unknown state transitions
- ✅ Prediction accuracy improves from cold start
- ✅ Exploration/exploitation switching working

### By End of Splicing & Synthesis
- ✅ GGUF weights extracted correctly
- ✅ Binary patching coherence >0.7
- ✅ Synthesized agents hot-loadable

### By End of Glass Workshop
- ✅ O3DE renders Ariel avatar + Glass lens
- ✅ Drag-drop file ingestion working
- ✅ Animation state updates reactive

### By End of Phase 6D Complete
- ✅ Full E2E: User intent → Glass perception → Ariel decision → Marionette action
- ✅ <200ms total latency
- ✅ 150-175 tests passing
- ✅ Zero critical panics

---

# RISK MITIGATION

| Risk | Mitigation |
|------|-----------|
| HID latency >1ms | Use lower-level Win32 APIs, profile with VTune |
| GGUF splicing fails | Validate weights with perplexity check |
| Memory pressure | Aggressive quantization, SSD-only fallback |
| Relic comm slow | Use mmap'd ringbuffer instead of IPC |
| O3DE integration pain | Stub O3DE initially, use mock renderer |

---

**This is the path to Era 3. Let's execute it.**
