// bench_animation.js - Triggers a continuous CSS transform/canvas animation for benchmarking
(() => {
  console.log("⚡ Starting visual benchmark animation scenario...");

  // Create an animated canvas or DOM box if none exists
  let box = document.getElementById("__tauridrive_bench_box");
  if (!box) {
    box = document.createElement("div");
    box.id = "__tauridrive_bench_box";
    box.style.position = "fixed";
    box.style.top = "50px";
    box.style.left = "50px";
    box.style.width = "100px";
    box.style.height = "100px";
    box.style.backgroundColor = "rgba(79, 70, 229, 0.85)";
    box.style.borderRadius = "12px";
    box.style.boxShadow = "0 10px 25px rgba(0, 0, 0, 0.2)";
    box.style.transition = "transform 0.05s linear";
    box.style.zIndex = "999999";
    document.body.appendChild(box);
  }

  let angle = 0;
  function animate() {
    angle += 4;
    const x = Math.sin(angle * Math.PI / 180) * 120;
    const y = Math.cos(angle * Math.PI / 180) * 60;
    if (box) {
      box.style.transform = `translate(${x}px, ${y}px) rotate(${angle}deg)`;
    }
    requestAnimationFrame(animate);
  }
  requestAnimationFrame(animate);

  return { status: "animation_running" };
})();
