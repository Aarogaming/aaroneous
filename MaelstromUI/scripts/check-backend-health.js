const fetch = require('node-fetch');
const BACKEND_CHECK_URL = "http://localhost:4222"; // Example for NATS broker
const MAX_RETRIES = 12;
const INTERVAL = 5000; // 5 seconds

const checkBackendHealth = async () => {
  let retries = 0;

  while (retries < MAX_RETRIES) {
    try {
      const res = await fetch(BACKEND_CHECK_URL);
      if (res.status === 200) {
        console.log("Backend is healthy!");
        process.exit(0);
      }
    } catch (e) {
      console.log(`Backend not ready. Retrying (${retries + 1}/${MAX_RETRIES})...`);
    }
    retries++;
    await new Promise((resolve) => setTimeout(resolve, INTERVAL));
  }

  console.error("Backend failed to start after maximum retries.");
  process.exit(1);
};

checkBackendHealth();