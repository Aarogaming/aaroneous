import React, { useRef } from "react";
import { Bot, BrainCircuit, Hexagon, Send } from "lucide-react";

type CommandCenterTabProps = {
  log: { sender: string; text: string }[];
  intent: string;
  setIntent: (value: string) => void;
  submitIntent: () => Promise<void>;
  dagTasks: { specialist: string; output: string; status: string }[];
};

function CommandCenterTab({ log, intent, setIntent, submitIntent, dagTasks }: CommandCenterTabProps) {
  const logEndRef = useRef<HTMLDivElement>(null);
  const dagEndRef = useRef<HTMLDivElement>(null);

  return (
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
  );
}

export default CommandCenterTab;