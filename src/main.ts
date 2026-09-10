import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

const IMAGE_FILTERS = [
  { name: "图片文件", extensions: ["png", "jpg", "jpeg", "webp", "bmp", "gif", "tiff", "tif"] },
];

type Files = string[];
const wmFiles: Files = [];
const cpFiles: Files = [];

interface ProgressPayload {
  done: number;
  total: number;
  current: string;
}

function $(sel: string): HTMLElement {
  const el = document.querySelector(sel);
  if (!el) throw new Error(`element not found: ${sel}`);
  return el as HTMLElement;
}

function bindValue(id: string, fmt: (v: number) => string) {
  const input = $(id) as HTMLInputElement;
  const update = () => {
    const label = input.parentElement?.querySelector(".val") as HTMLElement | null;
    if (label) label.textContent = fmt(Number(input.value));
  };
  input.addEventListener("input", update);
}

bindValue("#wm-size", (v) => String(v));
bindValue("#wm-opacity", (v) => `${v}%`);
bindValue("#wm-margin", (v) => String(v));
bindValue("#wm-spacing", (v) => String(v));
bindValue("#cp-quality", (v) => String(v));

$("#wm-tile").addEventListener("change", (e) => {
  const checked = (e.target as HTMLInputElement).checked;
  $("#wm-spacing-wrap").classList.toggle("hidden", !checked);
});

document.querySelectorAll(".tab").forEach((tab) => {
  tab.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach((t) => t.classList.remove("active"));
    document.querySelectorAll(".panel").forEach((p) => p.classList.remove("active"));
    tab.classList.add("active");
    const target = (tab as HTMLElement).dataset.tab;
    $(`#panel-${target}`).classList.add("active");
  });
});

async function pickImages(list: Files, countId: string): Promise<void> {
  const picked = await open({ multiple: true, directory: false, filters: IMAGE_FILTERS });
  if (!picked) return;
  const paths = typeof picked === "string" ? [picked] : picked;
  list.length = 0;
  list.push(...paths);
  $(countId).textContent = `已选择 ${list.length} 张`;
}

$("#wm-pick").addEventListener("click", () => pickImages(wmFiles, "#wm-count"));
$("#cp-pick").addEventListener("click", () => pickImages(cpFiles, "#cp-count"));

async function pickDir(inputId: string): Promise<void> {
  const dir = await open({ directory: true, multiple: false });
  if (dir) ($(inputId) as HTMLInputElement).value = dir;
}

$("#wm-output-pick").addEventListener("click", () => pickDir("#wm-output"));
$("#cp-output-pick").addEventListener("click", () => pickDir("#cp-output"));

interface StatusUI {
  bar: HTMLElement;
  text: HTMLElement;
  box: HTMLElement;
}

const status: StatusUI = {
  bar: $("#progress-bar"),
  text: $("#status-text"),
  box: $("#status"),
};

function setBusy(busy: boolean) {
  document.querySelectorAll<HTMLButtonElement>("button.primary").forEach((b) => (b.disabled = busy));
  if (busy) status.box.classList.remove("hidden");
  else status.box.classList.add("hidden");
}

function showError(msg: string) {
  status.box.classList.remove("hidden");
  status.bar.style.width = "0%";
  status.text.textContent = `出错了：${msg}`;
  setBusy(false);
}

let unlistenProgress: (() => void) | null = null;

async function runTask(
  command: string,
  opts: unknown,
  doneText: (n: number) => string,
): Promise<void> {
  if (unlistenProgress) {
    unlistenProgress();
    unlistenProgress = null;
  }
  unlistenProgress = await listen<ProgressPayload>("progress", (event) => {
    const { done, total, current } = event.payload;
    const pct = total > 0 ? Math.round((done / total) * 100) : 0;
    status.bar.style.width = `${pct}%`;
    status.text.textContent = `正在处理 ${done}/${total}：${current}`;
  });

  setBusy(true);
  status.bar.style.width = "0%";
  status.text.textContent = "正在处理…";
  try {
    const results = await invoke<string[]>(command, { opts });
    status.bar.style.width = "100%";
    status.text.textContent = doneText(results.length);
  } catch (e) {
    showError(String(e));
    return;
  }
  setBusy(false);
  if (unlistenProgress) {
    unlistenProgress();
    unlistenProgress = null;
  }
}

function currentVal(id: string): string {
  return ($(id) as HTMLInputElement).value;
}

$("#wm-start").addEventListener("click", async () => {
  if (wmFiles.length === 0) {
    showError("请先选择图片");
    return;
  }
  await runTask(
    "add_watermark",
    {
      files: wmFiles,
      outputDir: currentVal("#wm-output"),
      text: currentVal("#wm-text"),
      fontSize: Number(currentVal("#wm-size")),
      color: currentVal("#wm-color"),
      opacity: Number(currentVal("#wm-opacity")) / 100,
      position: ($("#wm-position") as HTMLSelectElement).value,
      margin: Number(currentVal("#wm-margin")),
      tile: ($("#wm-tile") as HTMLInputElement).checked,
      spacing: Number(currentVal("#wm-spacing")),
    },
    (n) => `完成！已为 ${n} 张图片添加水印`,
  );
});

$("#cp-start").addEventListener("click", async () => {
  if (cpFiles.length === 0) {
    showError("请先选择图片");
    return;
  }
  await runTask(
    "compress_images",
    {
      files: cpFiles,
      outputDir: currentVal("#cp-output"),
      quality: Number(currentVal("#cp-quality")),
      format: ($("#cp-format") as HTMLSelectElement).value,
    },
    (n) => `完成！已压缩 ${n} 张图片`,
  );
});