# Architectural Constraints for Aaroneous Substrate

## 🚫 Anti-Pattern: Inline Dependency Construction

Language models naturally gravitate toward **inlined instantiation** because it minimizes token uncertainty and requires zero knowledge of the broader dependency graph. This must be explicitly forbidden.

### The Problem

When models generate code without strict architectural bounds, they default to concrete monoliths:

```rust
// ❌ ANTI-PATTERN: Component tightly coupled and hardcoded inside another
pub struct Orchestrator {
    scheduler: ConcreteScheduler,
    circuit_breaker: ConcreteCircuitBreaker,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            // Directly constructed internally; completely rigid
            scheduler: ConcreteScheduler::new(100),  // ❌ VIOLATION
            circuit_breaker: ConcreteCircuitBreaker::new(5),  // ❌ VIOLATION
        }
    }
}
```

### Why This Happens

1. **Token Greediness**: Generating inline construction requires fewer tokens than defining traits and interfaces
2. **Context Window Laziness**: Models prefer local completeness over modular separation
3. **Training Bias**: Most public code is poorly modularized (tutorial/prototype style)

## ✅ Solution: Component Composition via Trait Boundaries

### 1. Constructor Injection Mandate

**Rule**: Never call `SubComponent::new()` inside a constructor or method of another component. All dependencies must be passed as arguments.

```rust
// ✅ CORRECT: Orchestrator coordinates parts of a whole, owning nothing concrete
pub trait JobScheduler {
    fn dispatch_tick(&mut self) -> Result<(), DispatchError>;
}

pub trait SafetyBreaker {
    fn is_tripped(&self) -> bool;
}

pub struct Orchestrator<S: JobScheduler, B: SafetyBreaker> {
    scheduler: S,
    breaker: B,
}

impl<S: JobScheduler, B: SafetyBreaker> Orchestrator<S, B> {
    // Injected at construction; individual parts are composed from outside
    pub fn new(scheduler: S, breaker: B) -> Self {
        Self { scheduler, breaker }  // ✅ Dependencies passed in, not constructed here
    }

    pub fn tick(&mut self) -> Result<(), DispatchError> {
        if self.breaker.is_tripped() {
            return Err(DispatchError::BreakerTripped);
        }
        self.scheduler.dispatch_tick()  // ✅ Only calls trait methods
    }
}
```

### 2. Slot-Map / Handle-Based System Architecture

For zero-allocation engines, components shouldn't own each other through pointers:

```rust
// State and components live independently in static storage
pub struct SystemContext {
    pub schedulers: [SchedulerState; 4],
    pub breakers: [BreakerState; 4],
}

// Orchestration is purely behavioral coordination over plain data
pub struct OrchestratorRouter;

impl OrchestratorRouter {
    pub fn step(ctx: &mut SystemContext, scheduler_idx: usize, breaker_idx: usize) {
        if ctx.breakers[breaker_idx].tripped {
            return;
        }
        ctx.schedulers[scheduler_idx].process_next();  // ✅ No ownership, just coordination
    }
}
```

## 📋 Implementation Guidelines

### Module Communication Rules

Components must communicate ONLY through:
1. **Public trait methods** (zero-cost static dispatch)
2. **POD message types** (fixed-layout structs for cross-module boundaries)
3. **Function pointers/trait objects** (for dynamic behavior)

**🚫 Forbidden:**
- Direct struct references across module boundaries
- Private fields accessed by other components
- Inline dependency construction (`Component::new()` called internally)

**✅ Required:**
- All cross-component communication via trait methods
- Dependencies injected, not constructed internally
- State coordination via external orchestrators, not embedded logic

## 🔍 Static Analysis: Detecting Violations

Add these negative tests to catch architectural violations:

```rust
// Add to test suite: detect inline instantiation violations
#[test]
fn no_inline_dependency_construction() {
    let source = include_str!("orchestrator.rs");
    
    // Match patterns like `scheduler: ConcreteScheduler::new()`
    assert!(!source.contains("Scheduler::new()"), 
            "Found inline Scheduler construction - violates dependency injection");
    assert!(!source.contains("Breaker::new()"), 
            "Found inline Breaker construction - violates dependency injection");
}

#[test]
fn no_direct_struct_cross_module_access() {
    // Ensure modules don't reference concrete types from other modules directly
    // Only trait references should cross boundaries
}
```

## 🎯 Verification Checklist

Before committing any code:

- [ ] No `SubComponent::new()` calls inside parent constructors
- [ ] All dependencies passed as function arguments or injected via traits
- [ ] Cross-module communication uses only trait methods or POD types
- [ ] Components don't hold references to other components' internal state
- [ ] Orchestrators coordinate state, they don't construct components

## 📂 Application to Existing Codebase

### crates/orchestration_plane

The `github_poller` module must follow these constraints:
- `GitHubPoller` should not contain concrete implementations of schedulers/breakers
- Dependencies (e.g., API clients, job handlers) must be injected via traits
- No inline construction in `new()` or `poll_once()` methods

### New Components

All new code in the substrate must:
1. Define trait interfaces first
2. Implement concrete types separately
3. Use constructor injection for composition
4. Pass static analysis checks for architectural violations
