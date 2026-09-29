// smoke_test.js - Sample tauridrive automation script
(() => {
  console.log("🚀 tauridrive smoke test starting...");

  // Verify DOM is ready
  const title = document.title;
  console.log(`Document title: ${title}`);

  // Query and interact with UI elements
  const btn = document.querySelector('button') || document.body;
  if (btn) {
    const rect = btn.getBoundingClientRect();
    console.log(`Found interactive target: bounds=(${rect.left}, ${rect.top}, ${rect.width}x${rect.height})`);
  }

  // Return a summary object to the runner
  return {
    success: true,
    title,
    url: window.location.href,
    timestamp: Date.now()
  };
})();
