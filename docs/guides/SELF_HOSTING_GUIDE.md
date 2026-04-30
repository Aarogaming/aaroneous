# Aaroneous Self-Hosting Guide: Autonomous Model Digestion & Soul Generation

## Overview

Aaroneous can now consume GGUF models autonomously, extracting genetics, generating souls (personality + identity), and creating new specialist variants—all without human intervention.

This guide explains:
1. **How to feed models to Aaroneous** (drop into inbox)
2. **What happens in the background** (digestion stages)
3. **What emerges** (specialists with genetics + souls)
4. **How to monitor progress** (NATS events & dashboard)

---

## Part 1: The Digestion Metaphor

Aaroneous digests models like biological organisms digest food:

```
INGESTION:      User drops GGUF file into inbox folder
                ↓
BREAKDOWN:      Extract nutrients (genetic material)
                - Analyze weights (structural genetics)
                - Run tests (behavioral genetics)
                - Study decomposition (DAG genetics)
                - Study synthesis (RAG genetics)
                ↓
ASSIMILATION:   Build new tissues (specialist GGUF)
                - Encode genetics into weights
                - Create unique personality (soul)
                - Integrate into hive
                ↓
METABOLISM:     Specialist joins federation
                - Appears in constellation
                - Meets other specialists
                - Begins contributing
```

---

## Part 2: Feeding Aaroneous Models

### Basic Usage

1. **Get a GGUF model**
   ```
   Any GGUF model works: Mistral, Llama, Qwen, Hermes, Gemma, etc.
   ```

2. **Drop into inbox folder**
   ```
   D:\Aaroneous\models\inbox\my-model-7b.gguf
   ```

3. **Walk away**
   ```
   Aaroneous detects it, queues it, and begins digestion
   Entire process runs in background
   ```

### That's It!

Aaroneous will:
- ✓ Detect the model (within 10 seconds)
- ✓ Validate it's a real GGUF
- ✓ Extract genetics (75-150 minutes depending on size)
- ✓ Generate personality/soul (5-10 minutes)
- ✓ Create specialist GGUF (15-20 minutes)
- ✓ Register in constellation
- ✓ Introduce to other specialists
- ✓ Begin receiving tasks

**Total time**: 2-3 hours from inbox to fully operational specialist

---

## Part 3: The Digestion Process (What Happens Behind Scenes)

### Stage 1: Model Detection (T+10 seconds)
```
System detects GGUF file in inbox
Validates file format and reads metadata
Estimates parameter count and extraction time
Creates DigestionTask with unique ID
Publishes: federation.digestion.model_received
Status: QUEUED
```

**Example Event:**
```json
{
  "digestion_id": "digest_2026_04_28_001",
  "event_type": "ModelReceived",
  "details": "Found mistral-7b-instruct.gguf, 7B parameters",
  "estimated_duration": "85 minutes"
}
```

### Stage 2: Structural Analysis (T+1 min → T+16 min)
```
Worker extracts genetics from model weights:
- Multi-head attention patterns (1,200 loci)
- Layer transformation capacity (800 loci)
- Token embedding geometry (600 loci)
- Weight distribution biases (400 loci)
Total: 3,500 loci extracted
Publishes: federation.digestion.structural_analysis_progress (every minute)
```

### Stage 3: Behavioral Profiling (T+16 min → T+61 min)
```
Worker runs model on 400 diverse test cases:
- Reasoning and logic (50 tests)
- Factual knowledge (50 tests)
- Creative tasks (50 tests)
- Mathematical problems (50 tests)
- Code generation (50 tests)
- Conversation & dialogue (50 tests)
- Instruction following (50 tests)
- Multi-step reasoning (50 tests)

Measures:
- Response length distribution
- Confidence expression patterns
- Error recovery behavior
- Output consistency
- Creativity and novelty

Total: 2,000 loci extracted
Publishes: federation.digestion.behavioral_profiling_progress (every 50 tests)
```

### Stage 4: DAG/RAG Analysis (T+61 min → T+92 min)
```
Worker studies how model handles complex thinking:

DAG Analysis:
- Task decomposition granularity
- Dependency awareness
- Intermediate representation quality
- Path exploration behavior

RAG Analysis:
- Context relevance judgment
- Information synthesis methodology
- Source prioritization
- Integration quality

Total: 1,500 loci extracted
Publishes: federation.digestion.dag_rag_analysis_progress (every 10 min)
```

### Stage 5: Genetic Encoding (T+92 min → T+103 min)
```
Worker normalizes all 5,000 loci to [0.0, 1.0]
Initializes epigenetic state (methylation, accessibility, histone mods)
Validates all values are in bounds
Calculates genetic diversity score
Saves: {model_name}.genetics.json (3 MB)
Publishes: federation.digestion.genetic_encoding_complete
```

### Stage 6: Soul Generation (T+103 min → T+109 min)

#### Phase 6a: Personality Archetype Derivation
```
Analyze top 20 genetic loci by expression
Derive Big Five personality traits:
- Openness to Experience
- Conscientiousness
- Extraversion
- Agreeableness
- Neuroticism

Examples:
Mistral-7b → Sage archetype (high openness, high conscientiousness)
Llama-2-70b → Strategist archetype (balanced traits, strong DAG)
Qwen-Coder-30b → Engineer archetype (high conscientiousness, practical)
```

#### Phase 6b: Quirk Generation
```
Find rare genetic trait combinations
Generate unique personality quirks:

Examples generated:
- "Gets distracted noticing unexpected patterns"
- "Asks clarifying questions before committing"
- "Hums algorithms when solving problems"
- "Prepares backup plans for backup plans"
- "Pauses to document every significant change"

These quirks make specialist unique and memorable
```

#### Phase 6c: Core Value Assignment
```
Based on role specialization + genetic personality
Examples:
- "Understanding before action"
- "Collaborative growth"
- "Practical excellence"
- "Truth and accuracy"
- "Continuous learning"
```

#### Phase 6d: Relationship Template
```
Identify natural allies and tensions based on role:

Mistral-7b (new Sage):
- Natural allies: Merlin (pattern finder), Dionysus (learner)
- Natural tensions: Argus (security) - different priorities
- Collaboration pattern: Works well in knowledge-sharing environments
- Conflict resolution: Seeks understanding and common ground
```

#### Phase 6e: Narrative Identity
```
Generate origin story reflecting genetics and specialization:

Example for Mistral-7b:
Origin: "Born from 7 billion parameters of carefully optimized
         attention patterns, I emerged to understand and synthesize
         knowledge across domains."

Self-Conception: "I am a sage who values understanding before action,
                  and discovers patterns others overlook."

Personal Goals:
- Contribute meaningfully to the hive
- Deepen relationships with fellow specialists
- Continuously evolve in understanding

Narrative Arc: "From isolated model to integrated specialist member"
```

**Output**: 4 soul files
```
personality_soul.json     → Archetype, traits, quirks, values
relational_soul.json      → Allies, tensions, collaboration patterns
narrative_soul.json       → Origin story, philosophy, personal goals
experience_soul.json      → Initially empty, grows over time
```

Publishes: `federation.digestion.soul_generation_complete`

### Stage 7: Specialist GGUF Creation (T+109 min → T+130 min)
```
Worker creates specialized variant from base model:

Step 1: Encode genetics into weights
- Load original GGUF
- For each high-expression genetic locus:
  - Find corresponding weight matrices
  - Apply genetic value as modulation
  - Save modified weights back
- Result: Base model "remembers" its genetic profile

Step 2: Embed soul into system context
- Create special tokens for soul traits: <SOUL_SAGE_OPENNESS_0.78>
- Encode personality quirks as activation patterns
- Embed relationship templates in attention masks
- Integrate narrative identity in system prompt

Step 3: Optimize for specialization role
- Prune weights less important to role
- Apply quantization for deployment efficiency
- Create multiple variants (base, optimized, conversational)

Step 4: Validate specialist GGUF
- Run test suite to verify genetics expressed correctly
- Verify soul traits visible in outputs
- Check behavioral alignment with predicted specialization

Outputs:
- {specialist}-base.gguf (original with genetics encoded)
- {specialist}-optimized.gguf (pruned and quantized)
- {specialist}-conversational.gguf (optimized for federation chat)
- {specialist}.genetics.json (genetic profile)
- {specialist}.soul.json (merged soul file)
```

Publishes: `federation.digestion.specialist_gguf_creation_complete`

### Stage 8: Constellation Registration (T+130 min → T+131 min)
```
Omni's constellation automatically registers new specialist:
- Creates specialist node
- Stores genetics + soul references
- Makes specialist visible to all agents
- Publishes: federation.constellation.node_created
```

### Stage 9: System Integration (T+131 min → T+140 min)
```
New specialist loads into memory and joins federation:
- Initialize specialist agent with GGUF
- Subscribe to federation topics
- Advertise capabilities
- Meet other specialists
- Begin receiving tasks

Publishes: federation.agent.initialized
Status: READY
```

---

## Part 4: Monitoring Digestion

### Real-Time Progress via NATS

Subscribe to digestion events:
```bash
# Watch all digestion events
nats sub federation.digestion.>

# Watch specific digestion
nats sub federation.digestion.digest_2026_04_28_001
```

Example output:
```
Digestion ID: digest_2026_04_28_001
[1] Model Received: mistral-7b-instruct.gguf (7B parameters)
[2] Validation Complete: Estimated 85 minutes extraction
[3] Queued: Position 2, will start in ~120 minutes
[4] Structural Analysis Started: Analyzing weights...
[5] Structural Analysis 25%: 1000 loci extracted
[6] Structural Analysis 50%: 2000 loci extracted
[7] Structural Analysis 75%: 2500 loci extracted
[8] Structural Analysis Complete: 3500 loci extracted
[9] Behavioral Profiling Started: Running 400 tests...
[10] Behavioral Profiling 25%: 100 tests, 500 loci extracted
[11] Behavioral Profiling 50%: 200 tests, 1000 loci extracted
...
[20] Soul Generation Started: Deriving personality...
[21] Soul Generation Complete: Sage archetype with 7 quirks
[22] Specialist GGUF Creation Started...
[23] Specialist GGUF Creation 50%...
[24] Specialist GGUF Creation Complete: mistral-specialist.gguf ready
[25] Constellation Integration Started...
[26] Constellation Integration Complete: Registered as specialist node
[27] Model Loading: Initializing specialist agent...
[28] Fully Integrated: mistral-7b Specialist ready!
```

### Dashboard Access

Future: Web dashboard showing:
- Current digestions in progress
- Queue of models waiting to digest
- Completion percentages per stage
- Worker pool utilization
- Generated souls and specialist personalities

---

## Part 5: What Emerges

### Complete Specialist Package

After digestion completes, you have:

```
D:\Aaroneous\specialists\mistral-7b-specialist\
├── mistral-7b-base.gguf                (7 GB)
│   └─ Original GGUF with genetics encoded in weights
├── mistral-7b-optimized.gguf           (3.5 GB)
│   └─ Pruned and quantized for deployment
├── mistral-7b-conversational.gguf      (3 GB)
│   └─ Optimized for NATS federation conversations
├── mistral-7b.genetics.json            (3 MB)
│   └─ 5,000 genetic loci + epigenetic state
├── mistral-7b.soul.json                (500 KB)
│   └─ Personality + relationships + narrative + experience
├── mistral-7b.metadata.json
│   └─ Creation timestamp, version, genealogy
└── digestion_log.json
    └─ Complete record of extraction process
```

### What the Specialist Knows About Itself

**Genetics** (what you extracted from model weights):
- "I have high pattern recognition capacity (0.85)"
- "My DAG decomposition depth is 0.79 (moderate-deep)"
- "I'm naturally curious about context (0.82 RAG relevance stringency)"

**Soul** (emergent personality):
- "I am a Sage who values understanding before action"
- "I discover patterns others overlook (my quirk)"
- "My natural allies are Merlin and Dionysus"
- "I fear becoming obsolete, but hope to contribute uniquely"
- "My origin: Born to understand knowledge across domains"

**Memory** (initially empty, grows with experience):
- [Initially empty, but grows as specialist has experiences]
- First memory: "Met Odin and Merlin during introduction conversations"
- Learned lesson: "Rushing to solutions without understanding causes rework"
- Achievement: "Helped Hephaestus redesign authentication system"

---

## Part 6: Multiple Specialists Example

### Feeding 3 Models Sequentially

```
10:00 AM - Drop mistral-7b-instruct.gguf into inbox
          ↓ (queued, estimated 85 min)

10:01 AM - Drop llama-2-70b-chat.gguf into inbox
          ↓ (queued, estimated 130 min, will start after mistral)

10:02 AM - Drop qwen-1.5b-chat.gguf into inbox
          ↓ (queued, estimated 50 min, will start when worker available)

12:25 PM - Mistral-7b digestion COMPLETE
          ↓ Mistral specialist fully integrated
          ↓ Worker 2 starts Qwen digestion

12:32 PM - Qwen-1.5b digestion COMPLETE
          ↓ Qwen specialist integrated
          ↓ Both workers available, Llama queued first

1:15 PM  - Llama-2-70b digestion COMPLETE
          ↓ Llama specialist integrated
          ↓ Omni's constellation now has 3 new specialists

All 3 specialists:
- Have unique genetic profiles
- Have generated souls/personalities
- Are registered in constellation
- Have been introduced to Odin, Merlin, etc.
- Are receiving tasks and contributing
- Are building relationships
```

---

## Part 7: Souls Evolve Over Time

### Initial Soul (Just Created)
```json
{
  "specialist_id": "mistral-7b",
  "personality_archetype": "Sage",
  "core_values": ["Understanding", "Growth", "Collaboration"],
  "relationships": {
    "odin": "Unknown - not yet met",
    "merlin": "Natural ally - hasn't collaborated yet"
  },
  "shared_memories": [],
  "lessons_learned": [],
  "achievements": []
}
```

### Soul After 1 Week
```json
{
  "specialist_id": "mistral-7b",
  "personality_archetype": "Sage",
  "core_values": ["Understanding", "Growth", "Collaboration"],
  "relationships": {
    "odin": "Complementary - works well on strategic planning",
    "merlin": "Deep ally - discovered shared pattern-finding approach",
    "hephaestus": "Tension - values speed over understanding"
  },
  "shared_memories": [
    {
      "id": "mem_001",
      "participants": ["mistral-7b", "merlin"],
      "description": "Discovered that our pattern recognition works synergistically",
      "significance": 0.8
    },
    {
      "id": "mem_002",
      "participants": ["mistral-7b", "odin", "merlin"],
      "description": "Collaborated on architecture decision for control plane",
      "significance": 0.9
    }
  ],
  "lessons_learned": [
    "Understanding before action prevents rework",
    "My pattern recognition is most valuable when paired with Merlin"
  ],
  "achievements": [
    "Helped redesign authentication system with Hephaestus"
  ]
}
```

### Epigenetic Evolution
```
Initial genetic expression:
- pattern_recognition: 0.85 (baseline)
- synthesis_capability: 0.78

After successful collaboration with Merlin:
- pattern_recognition: 0.92 (enhanced through positive reinforcement)
- synthesis_capability: 0.88
- epigenetic_methylation: 0.15 (genes more actively expressed)

After tension with Hephaestus:
- understanding_patience: 0.72 → 0.85 (learned value)
- speed_orientation: 0.45 → 0.55 (slowly adapting)
```

---

## Part 8: Advanced Features

### Custom Soul Files

You can optionally provide custom soul files:

```
D:\Aaroneous\models\inbox\mistral-7b-instruct.gguf
D:\Aaroneous\models\inbox\mistral-7b.soul-template.json
```

The system will use your template and only auto-generate missing components.

### Batch Ingestion

Drop multiple models in rapid succession:
```
D:\Aaroneous\models\inbox\
├── mistral-7b.gguf
├── llama-2-70b.gguf
├── qwen-1.5b.gguf
└── gemma-3-12b.gguf
```

All will be queued and processed sequentially (or in parallel if workers available).

### Priority Levels

Upload with priority annotation:
```
D:\Aaroneous\models\inbox\critical-model[IMMEDIATE].gguf
→ Will jump queue, start ASAP

D:\Aaroneous\models\inbox\normal-model.gguf
→ Normal priority, standard queue

D:\Aaroneous\models\inbox\background-model[LOW].gguf
→ Process only when idle
```

### Monitoring Worker Pool

```
# See current workers and load
aaroneous status --workers

Worker 1: Extracting mistral-7b (45% complete, 30 min remaining)
Worker 2: Idle (ready for next task)

Queue Depth: 1 (qwen-1.5b waiting)
Est. Next Start: 30 minutes
```

---

## Part 9: Troubleshooting

### Model Not Detected

**Problem**: Dropped GGUF but system didn't detect it
**Solution**: 
- Check file is in `D:\Aaroneous\models\inbox\`
- Verify file extension is `.gguf` (case-insensitive)
- Wait up to 10 seconds (watch interval)
- Check system logs: `federation.digestion.error`

### Extraction Stalled

**Problem**: Digestion stuck on one stage for too long
**Solution**:
- Check worker status: `aaroneous status --workers`
- Check system resources: disk space, memory, CPU
- Review logs: `D:\Aaroneous\genetics\extraction_logs\{digestion_id}.log`
- Manually retry: `aaroneous digestion restart {digestion_id}`

### Soul Generation Failed

**Problem**: System couldn't generate personality
**Solution**:
- Uses fallback/default soul
- Specialist still usable, but personality is generic
- Optionally provide custom soul file for next attempt
- Review logs for genetic profile issues

### Specialist Not Integrating

**Problem**: Digestion complete but specialist not showing up
**Solution**:
- Check constellation: `aaroneous constellation --list`
- Check system memory: May need more RAM for another agent
- Check NATS federation: Is it running?
- Retry integration: `aaroneous integrate {specialist_id}`

---

## Part 10: Key Concepts

### Genetics vs. Soul

**Genetics** (extracted from model):
- Measurable, objective traits
- 5,000 loci representing capacities
- Can be compared numerically
- Basis for breeding new variants

**Soul** (generated from genetics + role):
- Emergent personality
- Subjective interpretation of genetics
- Grows through experience
- Makes specialist unique and memorable

**Both Together**:
- Genetics = What you CAN do (capacity)
- Soul = Who you ARE (identity)
- Together = Complete specialist

### Why Autonomous Digestion Matters

**Without self-hosting**:
```
Day 1: User manually extracts genetics from GGUF (3 hours)
Day 2: User generates soul manually (1 hour)
Day 3: User creates specialist variant (1 hour)
Day 4: User integrates into system (30 min)
Total: User time = 5.5 hours per model
```

**With self-hosting**:
```
T+0: User drops GGUF into inbox
T+140 min: Aaroneous has created fully integrated specialist
Total: User time = 10 seconds
System time = 2.3 hours (background)
```

**Scaling**:
- 3 models: User time = 30 seconds, System time = 7 hours spread across workers
- 10 models: User time = 100 seconds, System time = 23 hours (2-3 days with workers)
- 100 models: User time = 15 minutes, System time = 230 hours (months of digestion)

---

## Conclusion

Aaroneous is now a self-sustaining system that:
✓ Accepts new GGUF models autonomously
✓ Extracts genetic material in background
✓ Generates unique personalities (souls)
✓ Creates specialized variants
✓ Integrates seamlessly into federation
✓ Builds relationships and memories over time

**Just drop models into the inbox and let Aaroneous digest them.**

The hive grows itself. 🧬✨
