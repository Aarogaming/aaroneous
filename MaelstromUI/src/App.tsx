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
      <Titlebar />

      <div className="app-layout">
        <Sidebar activeTab={activeTab} setActiveTab={setActiveTab} status={status} />

        <div className="main-content">
          {activeTab === "command_center" && (
            <CommandCenterTab 
              log={log} 
              intent={intent} 
              setIntent={setIntent} 
              submitIntent={submitIntent} 
              dagTasks={dagTasks} 
            />
          )}

          {activeTab === "arsenal" && (
            <SABArsenalTab 
              arsenal={arsenal} 
              setAgentForm={setAgentForm} 
              setShowAgentForm={setShowAgentForm} 
              setActiveTab={setActiveTab} 
            />
          )}

          {activeTab === "agent_factory" && (
            <AgentFactoryTab 
              agentForm={agentForm} 
              setAgentForm={setAgentForm} 
              showAgentForm={showAgentForm} 
              setShowAgentForm={setShowAgentForm} 
              externalModels={externalModels} 
              factoryStatus={factoryStatus} 
              handleCreateAgent={handleCreateAgent} 
              handleImportDissect={handleImportDissect} 
              handleForgeSovereigns={handleForgeSovereigns} 
            />
          )}

{activeTab === "agentic_ops" && (
            <AgenticOpsTab 
              isRecording={isRecording} 
              toggleRecording={toggleRecording} 
              tasks={tasks} 
              fetchTasks={fetchTasks} 
              cancelTask={cancelTask} 
              routines={routines} 
              runRoutine={runRoutine} 
            />
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