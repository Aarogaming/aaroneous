# Aaroneous: From Vision to Working Code

**Purpose:** Transform the conceptual architecture into demonstrable, testable code

**Timeframe:** 2-4 weeks  
**Approach:** Revive dead code instead of rewriting  
**Success Metric:** 6 specialists coordinating with working learning loop

---

## Phase 1: Get Tests Running (Week 1)

### Issue: Test Binary Link Failure

**Current Problem:**
```
error: linking with `link.exe` failed: exit code: 1104
LINK : fatal error LNK1104: cannot open file 'a_run-11f922e85c77798b.exe'
```

**Root Cause:** Test binary is locked or build artifact conflict

**Solution:**
```bash
# Clean everything
cargo clean

# Try again with --offline to skip network
cargo test --lib federation::tests --offline

# If still fails, try single-threaded
cargo test --lib federation --test-threads=1
```

**Why this matters:** Can't test anything until tests compile

---

## Phase 2: Revive Entry 1 - Visionary Learning (Days 1-3)

### Step 1: Understand Current Implementation

**File:** `src/federation/specialists/visionary.rs`

Current state:
```rust
impl Visionary {
    fn learn_from_feedback(&mut self, feedback: &DesignFeedback) { ... }  // DEAD
    fn extract_engrams(&self) -> Vec<AestheticEngram> { ... }            // DEAD
    
    async fn propose(&self, context: &SpecialistContext) 
        -> Result<Vec<ProposedAction>, SpecialistError> {
        // Never uses learn_from_feedback or extract_engrams
    }
}
```

### Step 2: Make Visionary Mutable

**Change:**
```rust
// BEFORE
pub trait Specialist: Send + Sync {
    async fn execute(&self, decision: &Decision) -> Result<ExecutionResult, SpecialistError>;
}

// AFTER
pub trait Specialist: Send + Sync {
    async fn execute(&mut self, decision: &Decision) -> Result<ExecutionResult, SpecialistError>;
}
```

**Cascading changes:**
- All 6 specialists need `&mut self`
- Sentinel implementation changes
- All tests update

**Effort:** 2-3 hours (lots of grep+replace)

### Step 3: Integrate Learning in Execute

**In `visionary.rs`:**
```rust
#[async_trait]
impl Specialist for Visionary {
    async fn execute(&mut self, decision: &Decision) -> Result<ExecutionResult, SpecialistError> {
        // 1. Execute the decision
        let result = self._execute_design_decision(decision).await?;
        
        // 2. REVIVED: Learn from outcome
        let feedback = match result {
            ExecutionResult::Success => DesignFeedback {
                variant_id: decision.variant_id.clone(),
                approved: true,
                reason: Some("Successful execution".to_string()),
            },
            ExecutionResult::Failure => DesignFeedback {
                variant_id: decision.variant_id.clone(),
                approved: false,
                reason: Some("Execution failed".to_string()),
            },
        };
        
        self.learn_from_feedback(&feedback);  // NOW CALLED!
        
        // 3. Extract new engrams if enough data
        let new_engrams = self.extract_engrams();  // NOW CALLED!
        self.aesthetic_engrams.extend(new_engrams);
        
        Ok(result)
    }
}
```

### Step 4: Update Model Improvement Logic

**Current (simplistic):**
```rust
fn learn_from_feedback(&mut self, feedback: &DesignFeedback) {
    self.feedback_history.push(feedback.clone());
    
    let approval_rate = self.feedback_history.iter()
        .filter(|f| f.approved).count() as f32 
        / self.feedback_history.len() as f32;
    
    self.model_improvement_score = approval_rate;
}
```

**Improved (with confidence boost):**
```rust
fn learn_from_feedback(&mut self, feedback: &DesignFeedback) {
    self.feedback_history.push(feedback.clone());
    
    // Calculate approval rate
    let approval_rate = self.feedback_history.iter()
        .filter(|f| f.approved).count() as f32 
        / self.feedback_history.len() as f32;
    
    // Update model score
    self.model_improvement_score = approval_rate;
    
    // REVIVED: Boost confidence for next proposal
    if feedback.approved {
        self.next_proposal_confidence_boost = 0.1;  // 10% confidence bonus
    } else {
        self.next_proposal_confidence_boost = -0.05; // 5% penalty
    }
}
```

### Step 5: Use Learned Confidence in Proposals

**Update `propose()` to use learned confidence:**
```rust
async fn propose(&self, context: &SpecialistContext) 
    -> Result<Vec<ProposedAction>, SpecialistError> {
    
    let mut variants = self.generate_variants(3);
    
    // REVIVED: Use learned confidence boost
    for variant in &mut variants {
        variant.confidence += self.next_proposal_confidence_boost;
        variant.confidence = variant.confidence.clamp(0.0, 1.0);
    }
    
    // Create proposals from variants
    let proposals = variants
        .into_iter()
        .map(|v| ProposedAction {
            specialist: SpecialistId::Visionary,
            confidence: v.confidence,
            description: format!("Design: {}", v.description),
            // ...
        })
        .collect();
    
    Ok(proposals)
}
```

### Step 6: Write Integration Test

**File:** `src/federation/specialists/tests.rs` (create if doesn't exist)

```rust
#[cfg(test)]
mod visionary_learning_revival {
    use super::*;
    
    #[tokio::test]
    async fn visionary_learns_and_improves_confidence() {
        let mut visionary = Visionary::new();
        let context = SpecialistContext::default();
        
        // Get initial proposals
        let initial_proposals = visionary.propose(&context).await.unwrap();
        let initial_confidence = initial_proposals[0].confidence;
        println!("Initial confidence: {}", initial_confidence);
        
        // Simulate 3 successful executions
        for i in 0..3 {
            // Create a decision based on proposal
            let decision = Decision {
                specialist: SpecialistId::Visionary,
                action: format!("design_variant_{}", i),
                // ... other fields
            };
            
            // Execute (which now learns!)
            visionary.execute(&decision).await.unwrap();
            
            // Give feedback
            visionary.learn_from_feedback(&DesignFeedback {
                variant_id: format!("v{}", i),
                approved: true,
                reason: Some("User liked design".to_string()),
            });
        }
        
        // Get new proposals after learning
        let improved_proposals = visionary.propose(&context).await.unwrap();
        let improved_confidence = improved_proposals[0].confidence;
        println!("Improved confidence: {}", improved_confidence);
        
        // ASSERTION: Confidence should have improved
        assert!(
            improved_confidence > initial_confidence,
            "Expected confidence {} > {}", 
            improved_confidence, 
            initial_confidence
        );
    }
    
    #[tokio::test]
    async fn visionary_extracts_aesthetic_engrams() {
        let mut visionary = Visionary::new();
        
        // Initially no engrams
        assert_eq!(visionary.aesthetic_engrams.len(), 0);
        
        // Give feedback on multiple designs
        for _ in 0..5 {
            visionary.learn_from_feedback(&DesignFeedback {
                variant_id: "design-1".to_string(),
                approved: true,
                reason: None,
            });
        }
        
        // Extract engrams
        let engrams = visionary.extract_engrams();
        
        // Should have identified patterns
        assert!(engrams.len() > 0, "Should extract at least one engram");
        assert!(engrams[0].confidence > 0.5, "Pattern should have confidence");
        
        // Add to specialist
        visionary.aesthetic_engrams.extend(engrams);
        
        // Next proposal should use engrams
        let context = SpecialistContext::default();
        let proposals = visionary.propose(&context).await.unwrap();
        assert!(proposals.len() > 0);
    }
}
```

### Step 7: Run and Debug

```bash
# First just compile
cargo build --lib

# Then run just this test
cargo test visionary_learns_and_improves_confidence --lib -- --nocapture

# If it passes, you've revived the learning!
```

**Expected output:**
```
Initial confidence: 0.75
Improved confidence: 0.85

test visionary_learns_and_improves_confidence ... ok
test visionary_extracts_aesthetic_engrams ... ok
```

---

## Phase 3: Verify All 6 Specialists Work (Days 4-5)

### Check Each Specialist

```rust
#[tokio::test]
async fn all_specialists_implement_trait_correctly() {
    let specialists: Vec<Box<dyn Specialist>> = vec![
        Box::new(Visionary::new()),
        Box::new(Omnipresent::new()),
        Box::new(Symbiotic::new()),
        Box::new(Phygital::new()),
        Box::new(Archivist::new()),
    ];
    
    let context = SpecialistContext::default();
    
    for specialist in specialists {
        // Each should be able to propose
        let proposals = specialist.propose(&context).await;
        assert!(proposals.is_ok(), "Specialist should propose");
        
        // Each should have capabilities
        let caps = specialist.capabilities();
        assert!(caps.len() > 0, "Specialist should declare capabilities");
    }
}
```

### Test Sentinel Arbitration

```rust
#[tokio::test]
async fn sentinel_arbitrates_specialist_proposals() {
    let mut hive = Hive::new();
    
    // Add all specialists
    hive.register_specialist(Visionary::new());
    hive.register_specialist(Omnipresent::new());
    // ... etc
    
    // Get proposals from all
    let all_proposals = hive.get_all_proposals().await;
    assert!(all_proposals.len() > 0, "Should have proposals from specialists");
    
    // Sentinel arbitrates
    let decision_result = hive.sentinel.arbitrate().await;
    assert!(decision_result.is_ok());
    assert!(decision_result.unwrap().decisions_issued > 0);
}
```

---

## Phase 4: DNA Bank Integration (Days 6-7)

### Record and Extract Patterns

```rust
#[tokio::test]
async fn dna_bank_records_and_extracts_patterns() {
    let mut dna_bank = DNABank::new();
    
    // Record multiple executions
    for i in 0..10 {
        let event = DNAEvent {
            specialist: SpecialistId::Visionary,
            event_type: "proposal_accepted".to_string(),
            outcome: "success".to_string(),
            duration_ms: 100 + (i * 10),
            metadata: Default::default(),
        };
        dna_bank.record_event(&event).await.unwrap();
    }
    
    // Extract patterns after 3+ similar events
    let patterns = dna_bank.extract_patterns().await.unwrap();
    assert!(patterns.len() > 0, "Should extract patterns from repeated events");
    
    // Check pattern quality
    for pattern in patterns {
        assert!(pattern.confidence > 0.3, "Pattern should have confidence");
        assert!(pattern.occurrence_count >= 3, "Pattern from 3+ occurrences");
    }
}
```

---

## Phase 5: End-to-End Learning Loop (Day 8)

### Full Cycle Test

```rust
#[tokio::test]
async fn e2e_specialist_coordination_and_learning() {
    let mut hive = Hive::new();
    
    // Initialize all specialists
    hive.register_specialist(Visionary::new());
    hive.register_specialist(Omnipresent::new());
    hive.register_specialist(Symbiotic::new());
    hive.register_specialist(Phygital::new());
    hive.register_specialist(Archivist::new());
    
    // Measure initial state
    let initial_stats = hive.get_stats().await;
    println!("Initial: {} proposals generated", initial_stats.total_proposals);
    
    // Run coordination loop 5 times
    for iteration in 0..5 {
        // 1. PROPOSE: All specialists submit ideas
        let proposals = hive.get_all_proposals().await;
        println!("Iteration {}: {} proposals", iteration, proposals.len());
        assert!(proposals.len() > 0);
        
        // 2. ARBITRATE: Sentinel picks best
        let arb_result = hive.sentinel.arbitrate().await.unwrap();
        println!("  Decisions issued: {}", arb_result.decisions_issued);
        assert!(arb_result.decisions_issued > 0);
        
        // 3. EXECUTE: Winners execute
        let outcomes = hive.execute_decisions().await.unwrap();
        println!("  Executions: {} successful", outcomes.iter().filter(|o| o.success).count());
        
        // 4. LEARN: Record and analyze
        for outcome in &outcomes {
            hive.dna_bank.record_event(&outcome.event).await.unwrap();
        }
        
        // 5. IMPROVE: Extract patterns
        let patterns = hive.dna_bank.extract_patterns().await.unwrap();
        println!("  Patterns extracted: {}", patterns.len());
        
        // Update specialist confidence based on patterns
        hive.update_specialist_confidence(&patterns).await.unwrap();
    }
    
    // Measure improvement
    let final_stats = hive.get_stats().await;
    println!("\nFinal: {} proposals generated", final_stats.total_proposals);
    println!("Success rate: {:.1}%", final_stats.success_rate * 100.0);
    
    // ASSERTIONS
    assert!(final_stats.total_proposals > initial_stats.total_proposals,
            "Should generate more proposals as specialists learn");
    
    assert!(final_stats.success_rate >= 0.7,
            "Should have >70% success rate with learning");
}
```

**Expected output:**
```
Iteration 0: 5 proposals
  Decisions issued: 1
  Executions: 1 successful
  Patterns extracted: 0
...
Iteration 4: 7 proposals
  Decisions issued: 1
  Executions: 1 successful
  Patterns extracted: 3

Final: 31 proposals generated
Success rate: 88.2%
```

---

## Validation Checklist

### Week 1 End
- [ ] Tests compile (cargo check passes)
- [ ] Visionary learning test passes
- [ ] All 6 specialists implement Specialist trait
- [ ] Sentinel arbitration works

### Week 2 End
- [ ] DNA Bank records events
- [ ] Pattern extraction works
- [ ] Confidence is updated based on patterns
- [ ] End-to-end loop completes 5 iterations
- [ ] Success rate improves over iterations

### Week 3 End (Bonus)
- [ ] Memory reflection integrated
- [ ] Skill evolution connected
- [ ] Consolidation of memory systems
- [ ] All dead code either revived or documented

---

## Roadblocks & Solutions

### Roadblock 1: Test Binary Won't Link
```
error: LNK1104: cannot open file 'a_run-11f922e85c77798b.exe'
```

**Solution:**
- Kill any running processes using the exe
- Clean build artifacts: `cargo clean`
- Try --offline: `cargo test --offline`

### Roadblock 2: Sentinel Not Issuing Decisions
**Solution:** Check `is_viable()` logic in Proposal - might be rejecting all

### Roadblock 3: DNA Bank Patterns Have 0 Confidence
**Solution:** Ensure pattern extraction algorithm actually calculates confidence from events

### Roadblock 4: Learning Doesn't Improve Proposals
**Solution:** Check that `model_improvement_score` is actually used in `propose()`

---

## Success Criteria

Your project is successful when:

1. ✅ **Code compiles** without linking errors
2. ✅ **Core tests pass** (visionary learning, sentinel arbitration)
3. ✅ **Specialists coordinate** (all 6 propose, Sentinel picks winner, winner executes)
4. ✅ **Learning works** (initial proposals vs proposals after 5 iterations show improved confidence)
5. ✅ **DNA Bank functional** (events recorded, patterns extracted with >0 confidence)

You don't need:
- ❌ Enterprise audit logging (future)
- ❌ Multi-hive federation (future)
- ❌ GPU acceleration (future)
- ❌ Full skill system (future)

Just **core coordination + learning**.

---

## Next Step

Pick **Phase 1, Step 1** and run:

```bash
cargo clean
cargo test --lib federation --offline
```

Report back with success or error messages. That's where we start.

