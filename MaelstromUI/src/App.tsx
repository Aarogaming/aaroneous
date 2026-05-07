import { useState, useEffect, useRef } from "react";
import { getCurrentWindow } from '@tauri-apps/api/window';
import { 
  Minus, Square, X, Hexagon, Activity, TerminalSquare, Database, 
  Settings, Bot, Send, BrainCircuit, Box, BoxSelect, Cpu, Layers 
} from "lucide-react";
import "./App.css";

const API_BASE = "http://localhost:8765";

function App() {
  const appWindow = getCurrentWindow();
  
  const [activeTab, setActiveTab] = useState("command_center");
  const [log, setLog] = useState<{ sender: string; text: string }[]>([
    { sender: "system", text: "Aaroneous Core online. How can the Hive assist you?" }
  ]);
  const [intent, setIntent] = useState("");
  const [status, setStatus] = useState("Connecting to Hive...");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [dagTasks, setDagTasks] = useState<{ specialist: string; output: string; status: string }[]>([]);
  const [arsenal, setArsenal] = useState<{ name: string; domain: string; kind: string }[]>([]);
  
  const [isRecording, setIsRecording] = useState(false);
  const [routines, setRoutines] = useState<{ id: string; name: string; time: string; status: string }[]>([]);

  // Dummy scheduled tasks for the UI
  const [tasks, setTasks] = useState<{ id: string; name: string; interval_secs: number; status: string }[]>([]);

  const [externalModels, setExternalModels] = useState<{ name: string; path: string; size_bytes: number; source: string }[]>([]);
  
  const cancelTask = async (id: string) => {
    try {
      await fetch(`${API_BASE}/scheduler/tasks/${id}`, { method: "DELETE" });
      fetchTasks();
    } catch (err) {
      console.error(err);
    }
  };

  const fetchTasks = () => {
    fetch(`${API_BASE}/scheduler/tasks`)
      .then(res => res.json())
      .then(data => {
        setTasks(data.tasks || []);
      })
      .catch(console.error);
  };

  const toggleRecording = async () => {
    const action = isRecording ? "stop" : "start";
    try {
      await fetch(`${API_BASE}/chimera/record`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action })
      });
      setIsRecording(!isRecording);
      if (action === "stop") {
        fetchRoutines(); // Refresh routines after stopping
      }
    } catch (err) {
      console.error(err);
    }
  };

  const fetchRoutines = () => {
    fetch(`${API_BASE}/chimera/routines`)
      .then(res => res.json())
      .then(data => {
        setRoutines(data.routines || []);
      })
      .catch(console.error);
  };

  const runRoutine = async (id: string) => {
    try {
      await fetch(`${API_BASE}/chimera/routines/${id}/run`, { method: "POST" });
      setLog(prev => [...prev, { sender: "system", text: `Triggered emulation playback for routine ${id}...` }]);
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    fetchRoutines();
    fetchTasks();
    const interval = setInterval(() => {
      fetchRoutines();
      fetchTasks();
    }, 10000);
    return () => clearInterval(interval);
  }, []);
  const [factoryStatus, setFactoryStatus] = useState("");
  
  const [agentForm, setAgentForm] = useState({ name: "", domain: "", gguf_path: "" });
  const [showAgentForm, setShowAgentForm] = useState(false);

  const logEndRef = useRef<HTMLDivElement>(null);
  const dagEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => { logEndRef.current?.scrollIntoView({ behavior: "smooth" }); }, [log]);
  useEffect(() => { dagEndRef.current?.scrollIntoView({ behavior: "smooth" }); }, [dagTasks]);

  // Init Session
  useEffect(() => {
    fetch(`${API_BASE}/sessions`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ user_name: "Commander" })
    })
      .then(res => res.json())
      .then(data => {
        setSessionId(data.session_id);
        setStatus("Connected to Hive");
      })
      .catch(err => {
        console.error("Failed to connect to backend", err);
        setStatus("Disconnected");
      });
  }, []);

  // Setup SSE
  useEffect(() => {
    if (!sessionId) return;
    const sse = new EventSource(`${API_BASE}/sessions/${sessionId}/results/stream`);
    sse.onopen = () => setStatus("Connected to Hive");
    sse.onerror = () => setStatus("Disconnected");

    sse.addEventListener("results", (e) => {
      const results = JSON.parse(e.data);
      for (const res of results) {
        setDagTasks(prev => [...prev, {
          specialist: res.specialist,
          output: res.output,
          status: res.status
        }]);

        if (res.specialist !== "Odin" || !res.output.includes('"tasks":')) {
          setLog(prev => [...prev, { sender: "agent", text: `[${res.specialist}] ${res.output}` }]);
        }
      }
    });

    const specSse = new EventSource(`${API_BASE}/specialists/stream`);
    specSse.addEventListener("specialist_update", (e) => {
      const update = JSON.parse(e.data);
      if (update.type === "guild_decomposition") {
        setLog(prev => [...prev, { sender: "system", text: `Odin generated DAG with ${update.decomposition?.tasks?.length || 0} tasks.` }]);
        const parsedTasks = update.decomposition.tasks || [];
        setDagTasks(parsedTasks.map((t: any) => ({
          specialist: t.assign_to || "Unknown",
          output: t.content,
          status: "Pending"
        })));
      }
    });

    return () => { sse.close(); specSse.close(); };
  }, [sessionId]);

  // Fetch Arsenal and External Models
  useEffect(() => {
    const fetchArsenal = () => {
      fetch(`${API_BASE}/specialists`)
        .then(res => res.json())
        .then(data => setArsenal(data.specialists || []))
        .catch(console.error);
    };
    
    const fetchExternalModels = () => {
      fetch(`${API_BASE}/models/external`)
        .then(res => res.json())
        .then(data => setExternalModels(data.models || []))
        .catch(console.error);
    };

    fetchArsenal();
    fetchExternalModels();
    const interval = setInterval(fetchArsenal, 10000);
    return () => clearInterval(interval);
  }, []);

  const handleCreateAgent = async () => {
    setFactoryStatus("Creating Agent...");
    try {
      const res = await fetch(`${API_BASE}/dynamic-specialists`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(agentForm)
      });
      const data = await res.json();
      if (res.ok) {
        setFactoryStatus(`Agent ${data.name} Created Successfully!`);
        setShowAgentForm(false);
      } else {
        setFactoryStatus(`Error: ${data.error}`);
      }
    } catch (err) {
      setFactoryStatus("Failed to create agent.");
    }
  };

  const handleImportDissect = async (modelPath: string) => {
    setFactoryStatus(`Importing ${modelPath.substring(modelPath.lastIndexOf('\\') + 1)}...`);
    try {
      const res = await fetch(`${API_BASE}/models/import`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ source: modelPath, auto_dissect: true, auto_register_sovereign: false })
      });
      const data = await res.json();
      setFactoryStatus(`Import Job Started: ${data.job_id}`);
    } catch (err) {
      setFactoryStatus("Import Failed.");
    }
  };

  const handleForgeSovereigns = async (modelPath: string) => {
    const filename = modelPath.split('\\').pop() || modelPath.split('/').pop() || "";
    const internalPath = `D:\\Aaroneous\\models\\${filename}`;
    setFactoryStatus(`Forging roster from ${filename}...`);
    try {
      const res = await fetch(`${API_BASE}/forge/crystallize-roster`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ source: internalPath, only: [] })
      });
      if (res.ok) setFactoryStatus(`Forge successful! GGUFs crystallized.`);
      else setFactoryStatus(`Forge request failed. Did you import first?`);
    } catch (err) {
      setFactoryStatus("Forge Failed.");
    }
  };

  const submitIntent = async () => {
    if (!intent.trim() || !sessionId) return;
    setLog(prev => [...prev, { sender: "user", text: intent }]);
    setIntent("");
    setStatus("Odin is analyzing intent...");
    setDagTasks([]);

    try {
      await fetch(`${API_BASE}/sessions/${sessionId}/intent`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ content: intent, priority: "High" })
      });
    } catch (err) {
      setLog(prev => [...prev, { sender: "system", text: "Error sending intent to Hive." }]);
      setStatus("Connected to Hive");
    }
  };

  return (
    <>
      <div className="titlebar">
        <div className="titlebar-title">
          <Hexagon size={14} color="var(--accent-solid)" />
          Aaroneous Command Center
        </div>
        <div className="titlebar-actions">
          <button className="titlebar-btn" onClick={() => appWindow.hide()}><Minus size={16} /></button>
          <button className="titlebar-btn" onClick={() => appWindow.toggleMaximize()}><Square size={14} /></button>
          <button className="titlebar-btn close" onClick={() => appWindow.hide()}><X size={16} /></button>
        </div>
      </div>

      <div className="app-layout">
        <div className="sidebar">
          <div className="sidebar-header">
            <div className="brand">
              <Hexagon size={24} color="var(--accent-solid)" />
              <h1>Maelstrom</h1>
            </div>
            <div className="hive-status">
              <div className={`status-dot ${status.includes("Disconnected") ? "offline" : "online"}`}></div>
              {status}
            </div>
          </div>
          
          <div className="nav-menu">
            <button className={`nav-item ${activeTab === "command_center" ? "active" : ""}`} onClick={() => setActiveTab("command_center")}>
              <TerminalSquare size={16} /> Orchestrator (1v1)
            </button>
            <button className={`nav-item ${activeTab === "arsenal" ? "active" : ""}`} onClick={() => setActiveTab("arsenal")}>
              <Layers size={16} /> SAB Arsenal
            </button>
            <button className={`nav-item ${activeTab === "agent_factory" ? "active" : ""}`} onClick={() => setActiveTab("agent_factory")}>
              <Cpu size={16} /> Synth DNA Forge
            </button>
            <button className={`nav-item ${activeTab === "agentic_ops" ? "active" : ""}`} onClick={() => setActiveTab("agentic_ops")}>
              <Activity size={16} /> Chimera Eye
            </button>
          </div>

          <div className="user-profile">
            <div className="avatar">AA</div>
            <div className="user-info">
              <span className="user-name">Commander</span>
              <span className="user-role">System Admin</span>
            </div>
          </div>
        </div>

        <div className="main-content">
          {activeTab === "command_center" && (
            <div className="chat-layout" style={{ padding: "32px" }}>
              <div className="chat-column">
                <div className="messages-area">
                  {log.map((msg, idx) => (
                    <div key={idx} className={`message-row ${msg.sender}`}>
                      {msg.sender !== "system" && (
                        <div className={`msg-avatar ${msg.sender === "user" ? "user-av" : "agent-av"}`}>
                          {msg.sender === "user" ? "U" : <Bot size={20} />}
                        </div>
                      )}
                      <div className="msg-bubble">{msg.text}</div>
                    </div>
                  ))}
                  <div ref={logEndRef} />
                </div>
                <div className="input-container">
                  <div className="input-box">
                    <input 
                      type="text" 
                      placeholder="Message the Hive (e.g. 'Odin, run a security scan...')" 
                      value={intent}
                      onChange={(e) => setIntent(e.target.value)}
                      onKeyDown={(e) => e.key === 'Enter' && submitIntent()}
                    />
                    <button className="send-btn" onClick={submitIntent}><Send size={16} /></button>
                  </div>
                </div>
              </div>

              <div className="dag-column">
                <div className="dag-panel">
                  <div className="dag-header">
                    <BrainCircuit size={18} /> Live DAG Execution
                  </div>
                  {dagTasks.length === 0 ? (
                    <div className="empty-state">
                      <Hexagon size={32} />
                      <span>No active task graph.</span>
                    </div>
                  ) : (
                    dagTasks.map((t, idx) => (
                      <div key={idx} className="dag-node">
                        <div className="dag-node-header">
                          <span className="dag-specialist">{t.specialist}</span>
                          <span className="dag-status">{t.status}</span>
                        </div>
                        <div className="dag-output">{t.output}</div>
                      </div>
                    ))
                  )}
                  <div ref={dagEndRef} />
                </div>
              </div>
            </div>
          )}

          {activeTab === "arsenal" && (
            <div className="tab-body">
              <div className="tab-header" style={{ padding: "0 0 24px 0", marginBottom: "24px" }}>
                <h2>Sovereign Agent Bundles</h2>
                <p>Dynamically compiled WASM plugins and Native DLLs actively loaded in memory.</p>
              </div>
              <div className="grid-layout">
                {arsenal.map((spec, idx) => (
                  <div key={idx} className="card">
                    <div className="card-header">
                      <h3 className="card-title"><Box size={18} /> {spec.name}</h3>
                      <span className={`badge ${spec.kind === "core" ? "native" : "wasm"}`}>{spec.kind}</span>
                    </div>
                    <p className="card-desc">{spec.domain}</p>
                    <div className="card-actions">
                      <button className="btn btn-secondary" onClick={() => {
                        setAgentForm({ name: spec.name, domain: spec.domain, gguf_path: "" });
                        setShowAgentForm(true);
                        setActiveTab("agent_factory");
                      }}>
                        <Settings size={14} /> Configure
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === "agent_factory" && (
            <div className="tab-body">
              <div className="tab-header" style={{ padding: "0 0 24px 0", marginBottom: "24px", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <h2>Synth DNA Forge</h2>
                  <p>Discover GGUF models, dissect neural structures, and forge new Sovereigns.</p>
                </div>
                {factoryStatus && (
                  <div style={{ color: "var(--accent-solid)", fontSize: "14px", fontWeight: 600 }}>{factoryStatus}</div>
                )}
              </div>
              
              {showAgentForm ? (
                <div className="form-group">
                  <h3><BoxSelect size={18} /> Spawn Custom Agent</h3>
                  <input type="text" className="input-field" placeholder="Agent Name (e.g. Architect)" value={agentForm.name} onChange={e => setAgentForm({...agentForm, name: e.target.value})} />
                  <input type="text" className="input-field" placeholder="Domain Expertise (e.g. coding)" value={agentForm.domain} onChange={e => setAgentForm({...agentForm, domain: e.target.value})} />
                  <input type="text" className="input-field" placeholder="GGUF File Path (Optional)" value={agentForm.gguf_path} onChange={e => setAgentForm({...agentForm, gguf_path: e.target.value})} />
                  <div style={{ display: "flex", gap: "12px" }}>
                    <button className="btn btn-primary" onClick={handleCreateAgent}>Deploy</button>
                    <button className="btn btn-secondary" onClick={() => setShowAgentForm(false)}>Cancel</button>
                  </div>
                </div>
              ) : (
                <button className="btn btn-primary" style={{ marginBottom: "24px" }} onClick={() => setShowAgentForm(true)}>
                  <BoxSelect size={16} /> Custom Deployment
                </button>
              )}
              
              <div className="form-group">
                <h3><Database size={18} /> HuggingFace Fast Sync</h3>
                <div style={{ display: "flex", gap: "12px" }}>
                  <button className="btn btn-secondary" onClick={() => handleImportDissect("hf://Qwen/Qwen2.5-Coder-1.5B-Instruct-GGUF")}>Qwen 1.5B Coder</button>
                  <button className="btn btn-secondary" onClick={() => handleImportDissect("hf://Qwen/Qwen2.5-7B-Instruct-GGUF")}>Qwen 7B Core</button>
                </div>
                <div className="input-box" style={{ marginTop: "16px" }}>
                  <input type="text" id="hf-input" placeholder="hf://Repo/Name" />
                  <button className="send-btn" onClick={() => {
                    const val = (document.getElementById("hf-input") as HTMLInputElement).value;
                    if (val) handleImportDissect(val);
                  }}><Send size={16} /></button>
                </div>
              </div>

              <h3>Local Model Discovery (LM Studio / Ollama)</h3>
              <div className="grid-layout" style={{ marginTop: "16px" }}>
                {externalModels.length === 0 ? (
                  <div className="empty-state">No local models detected.</div>
                ) : (
                  externalModels.map((model, idx) => (
                    <div key={idx} className="card">
                      <div className="card-header">
                        <h3 className="card-title"><Database size={16} /> {model.name}</h3>
                        <span className="badge external">{model.source}</span>
                      </div>
                      <div className="card-meta">{model.path}</div>
                      <p className="card-desc">Size: {(model.size_bytes / 1073741824).toFixed(2)} GB</p>
                      <div className="card-actions">
                        <button className="btn btn-secondary" onClick={() => handleImportDissect(model.path)}>Dissect</button>
                        <button className="btn btn-primary" onClick={() => handleForgeSovereigns(model.path)}>Forge</button>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>
          )}

          {activeTab === "agentic_ops" && (
            <div className="tab-body">
              <div className="tab-header" style={{ padding: "0 0 24px 0", marginBottom: "24px" }}>
                <h2>Agentic Operations</h2>
                <p>Record user actions via Chimera Eye and assign execution schedules to SAB plugins.</p>
              </div>

              <div className="chimera-control" style={{ marginBottom: "30px" }}>
                <h3><Activity size={18} style={{ marginRight: "8px" }} /> Chimera Eye Emulation</h3>
                <p>Record your screen clicks to generate training data for autonomous routines.</p>
                <button 
                  className={isRecording ? "btn btn-danger" : "btn btn-primary"} 
                  onClick={toggleRecording}
                >
                  {isRecording ? "Stop Recording" : "Start Recording Routine"}
                </button>
              </div>
              
              <h3>Recorded Routines & Schedules</h3>
              <div style={{ display: "flex", gap: "10px", marginTop: "16px", marginBottom: "16px" }}>
                <button className="btn btn-secondary" onClick={() => {
                  const intent = prompt("Enter task intent:");
                  const interval = parseInt(prompt("Enter interval in seconds:") || "60");
                  if (intent && interval) {
                    fetch(`${API_BASE}/scheduler/tasks`, {
                      method: "POST",
                      headers: { "Content-Type": "application/json" },
                      body: JSON.stringify({ name: "Custom Task", intent_content: intent, interval_secs: interval })
                    }).then(fetchTasks);
                  }
                }}>
                  + Add Scheduled Task
                </button>
              </div>
              <div className="grid-layout" style={{ marginTop: "16px" }}>
                {routines.length === 0 && tasks.length === 0 ? (
                  <div className="empty-state">No routines recorded.</div>
                ) : (
                  <>
                    {tasks.map((task) => (
                      <div key={task.id} className="card">
                        <div className="card-header">
                          <h3 className="card-title">{task.name}</h3>
                          <span className={`badge ${task.status === "Scheduled" ? "native" : "wasm"}`}>{task.status}</span>
                        </div>
                        <p className="card-desc">Window: {task.interval_secs ? `Every ${task.interval_secs}s` : 'Unknown'}</p>
                        <div className="card-actions">
                          <button className="btn btn-secondary">Edit</button>
                          <button className="btn btn-danger" onClick={() => cancelTask(task.id)}>Cancel</button>
                        </div>
                      </div>
                    ))}
                    {routines.map((routine, idx) => (
                      <div key={`r-${idx}`} className="card">
                        <div className="card-header">
                          <h3 className="card-title">{routine.name}</h3>
                          <span className="badge external">{routine.status}</span>
                        </div>
                        <p className="card-desc">Time: {routine.time}</p>
                        <div className="card-actions">
                          <button className="btn btn-secondary">Assign Schedule</button>
                          <button className="btn btn-primary" onClick={() => runRoutine(routine.id)}>Run Now</button>
                        </div>
                      </div>
                    ))}
                  </>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </>
  );
}

export default App;