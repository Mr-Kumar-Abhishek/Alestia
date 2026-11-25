import init, {
  alestia_version,
  expand_greek_bayer,
  natural_sort_lines,
  parse_catalog_object,
  parse_color_hex,
  r128_decode_base64,
  r128_encode_base64,
} from "./pkg/alestia_web.js";

function setOutput(el, text, isError = false) {
  el.textContent = text;
  el.classList.toggle("is-error", isError);
}

async function main() {
  const badge = document.getElementById("wasm-badge");

  try {
    await init();
    badge.textContent = alestia_version();
    badge.classList.add("ready");
  } catch (err) {
    badge.textContent = `WASM init failed: ${err}`;
    badge.classList.add("error");
    return;
  }

  const catalogInput = document.getElementById("catalog-input");
  const catalogOutput = document.getElementById("catalog-output");
  const updateCatalog = () => {
    try {
      setOutput(catalogOutput, parse_catalog_object(catalogInput.value), false);
    } catch (err) {
      setOutput(catalogOutput, String(err), true);
    }
  };
  catalogInput.addEventListener("input", updateCatalog);
  updateCatalog();

  const r128Number = document.getElementById("r128-number");
  const r128Encoded = document.getElementById("r128-encoded");
  const r128Base64 = document.getElementById("r128-base64");
  const r128Decoded = document.getElementById("r128-decoded");

  const updateEncode = () => {
    const num = Number(r128Number.value);
    const encoded = r128_encode_base64(Number.isFinite(num) ? num : 0);
    r128Encoded.textContent = encoded;
    r128Base64.value = encoded;
    updateDecode();
  };

  const updateDecode = () => {
    try {
      const value = r128_decode_base64(r128Base64.value.trim());
      setOutput(r128Decoded, String(value), false);
    } catch (err) {
      setOutput(r128Decoded, String(err), true);
    }
  };

  r128Number.addEventListener("input", updateEncode);
  r128Base64.addEventListener("input", updateDecode);
  updateEncode();

  const greekInput = document.getElementById("greek-input");
  const greekOutput = document.getElementById("greek-output");
  const updateGreek = () => {
    setOutput(greekOutput, expand_greek_bayer(greekInput.value), false);
  };
  greekInput.addEventListener("input", updateGreek);
  updateGreek();

  const colorInput = document.getElementById("color-input");
  const colorSwatch = document.getElementById("color-swatch");
  const colorOutput = document.getElementById("color-output");
  const updateColor = () => {
    try {
      const summary = parse_color_hex(colorInput.value);
      setOutput(colorOutput, summary, false);
      colorSwatch.style.backgroundColor = colorInput.value.trim();
    } catch (err) {
      setOutput(colorOutput, String(err), true);
      colorSwatch.style.backgroundColor = "transparent";
    }
  };
  colorInput.addEventListener("input", updateColor);
  updateColor();

  const sortInput = document.getElementById("sort-input");
  const sortOutput = document.getElementById("sort-output");
  const updateSort = () => {
    setOutput(sortOutput, natural_sort_lines(sortInput.value), false);
  };
  sortInput.addEventListener("input", updateSort);
  updateSort();
}

main();
