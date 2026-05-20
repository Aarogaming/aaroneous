import React from "react";
import { Activity } from "lucide-react";

type AgenticOpsTabProps = {
  isRecording: boolean;
  toggleRecording: () => Promise<void>;
  tasks: { id: string; name: string; interval_secs: number; status: string }[];
  fetchTasks: () => void;
  cancelTask: (id: string) => Promise<void>;
  routines: { id: string; name: string; time: string; status: string }[];
  runRoutine: (id: string) => Promise<void>;
};

function AgenticOpsTab({
  isRecording,
  toggleRecording,
  tasks,
  fetchTasks,
  cancelTask,
  routines,
  runRoutine,
}: AgenticOpsTabProps) {
  return (
    <div className="tab-body">
      <div className="tab-header" style={{ padding: "0 0 24px 0", marginBottom: "24px" }}>
        <h2>Agentic Operations</h2>
        <p>Record user actions via Chimera Eye and assign execution schedules to SAB plugins.</p>
      </div>

      <div className="chimera-control" style={{ marginBottom: "30px" }}>
        <h3>
          <Activity size={18} style={{ marginRight: "8px" }} /> Chimera Eye
          Emulation
        </h3>
        <p>Record your screen clicks to generate training data for autonomous routines.</p>
        <button
          className={isRecording ? "btn btn-danger" : "btn btn-primary"}
          onClick={toggleRecording}
        >
          {isRecording ? "Stop Recording" : "Start Recording Routine"}
        </button>
      </div>

      <h3>Recorded Routines & Schedules</h3>
      <div
        style={{
          display: "flex",
          gap: "10px",
          marginTop: "16px",
          marginBottom: "16px",
        }}
      >
        <button
          className="btn btn-secondary"
          onClick={() => {
            const intent = prompt("Enter task intent:");
            const interval = parseInt(prompt("Enter interval in seconds:") || "60");
            if (intent && interval) {
              fetch("http://localhost:8765/scheduler/tasks", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                  name: "Custom Task",
                  intent_content: intent,
                  interval_secs: interval,
                }),
              }).then(fetchTasks);
            }
          }}
        >
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
                  <span
                    className={`badge ${
                      task.status === "Scheduled" ? "native" : "wasm"
                    }`}
                  >
                    {task.status}
                  </span>
                </div>
                <p className="card-desc">
                  Window: {task.interval_secs ? `Every ${task.interval_secs}s` : "Unknown"}
                </p>
                <div className="card-actions">
                  <button className="btn btn-secondary">Edit</button>
                  <button
                    className="btn btn-danger"
                    onClick={() => cancelTask(task.id)}
                  >
                    Cancel
                  </button>
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
                  <button
                    className="btn btn-primary"
                    onClick={() => runRoutine(routine.id)}
                  >
                    Run Now
                  </button>
                </div>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}

export default AgenticOpsTab;