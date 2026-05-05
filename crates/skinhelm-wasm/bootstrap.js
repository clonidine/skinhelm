import init, { SkinhelmViewer } from "./pkg/skinhelm_wasm.js";

const status = document.getElementById("status");
const canvas = document.getElementById("viewer-canvas");
const skinFile = document.getElementById("skin-file");
const loadDefault = document.getElementById("load-default");
const toggleAnimation = document.getElementById("toggle-animation");
const toggleOverlays = document.getElementById("toggle-overlays");
const animationSpeed = document.getElementById("animation-speed");

function setStatus(message) {
  status.textContent = message;
}

async function main() {
  await init();
  const viewer = SkinhelmViewer.init();

  loadDefault.addEventListener("click", () => {
    try {
      viewer.load_default_skin();
    } catch (error) {
      setStatus(String(error));
    }
  });

  skinFile.addEventListener("change", async () => {
    const file = skinFile.files && skinFile.files[0];
    if (!file) {
      return;
    }

    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      viewer.load_skin_bytes(bytes);
    } catch (error) {
      setStatus(String(error));
    }
  });

  toggleAnimation.addEventListener("change", () => {
    viewer.set_animation_enabled(toggleAnimation.checked);
  });

  toggleOverlays.addEventListener("change", () => {
    viewer.set_overlays_enabled(toggleOverlays.checked);
  });

  animationSpeed.addEventListener("input", () => {
    viewer.set_animation_speed(Number(animationSpeed.value));
  });

  canvas.addEventListener("mousedown", (event) => {
    viewer.pointer_down(event.clientX, event.clientY);
  });

  window.addEventListener("mousemove", (event) => {
    viewer.pointer_move(event.clientX, event.clientY);
  });

  window.addEventListener("mouseup", () => {
    viewer.pointer_up();
  });

  canvas.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      viewer.wheel(event.deltaY);
    },
    { passive: false },
  );

  window.addEventListener("resize", () => {
    try {
      viewer.resize();
    } catch (error) {
      setStatus(String(error));
    }
  });

  const frame = (timestamp) => {
    try {
      viewer.render_frame(timestamp);
    } catch (error) {
      setStatus(String(error));
    }
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

main().catch((error) => setStatus(String(error)));
