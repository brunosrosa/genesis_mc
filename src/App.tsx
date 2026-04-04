import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [loading, setLoading] = useState(false);

  async function pingCore() {
    setLoading(true);
    try {
      const response = await invoke("genesis_ping", { payload: "Heartbeat check" });
      setGreetMsg(response as string);
    } catch (error) {
      setGreetMsg(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="flex min-h-screen flex-col items-center justify-center bg-background text-foreground p-4">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5 }}
        className="flex flex-col items-center space-y-8"
      >
        <h1 className="text-4xl font-bold tracking-tight text-primary">Genesis Mission Control</h1>
        <p className="text-muted-foreground text-center max-w-md">
          Sovereign OS Local Environment.
          <br />
          Click the button below to test the IPC bridge.
        </p>

        <motion.button
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.95 }}
          onClick={pingCore}
          disabled={loading}
          className="rounded-md bg-primary px-8 py-3 text-sm font-medium text-primary-foreground shadow transition-colors hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50"
        >
          {loading ? "Pinging..." : "Execute Ping"}
        </motion.button>

        {greetMsg && (
          <motion.div
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            className="mt-8 rounded-lg border border-border bg-card p-6 text-card-foreground shadow-sm max-w-xl w-full"
          >
            <p className="font-mono text-sm break-words">{greetMsg}</p>
          </motion.div>
        )}
      </motion.div>
    </main>
  );
}

export default App;
