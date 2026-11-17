<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";

type Point = { x: number; y: number };
type Material = "partition" | "brick" | "concrete";
type Wall = { from: Point; to: Point; material: Material };
export type WifiPlan = {
  width_cm: number;
  height_cm: number;
  access_points: Point[];
  walls: Wall[];
};

const props = defineProps<{
  saved?: WifiPlan;
  busy: boolean;
  catalog: { name: string; index: number; price: string }[];
  error: string;
}>();
const emit = defineEmits<{
  save: [WifiPlan];
  quote: [number, number];
  close: [];
}>();
const dialog = ref<HTMLDialogElement>();
const canvas = ref<HTMLCanvasElement>();
const width = ref((props.saved?.width_cm ?? 1200) / 100);
const height = ref((props.saved?.height_cm ?? 800) / 100);
const accessPoints = ref<Point[]>(
  props.saved?.access_points.map((point) => ({ ...point })) ?? [],
);
const walls = ref<Wall[]>(
  props.saved?.walls.map((wall) => ({ ...wall })) ?? [],
);
const mode = ref<"ap" | "wall">("ap");
const anchor = ref<Point>();
const material = ref<Material>("partition");
const item = ref<number | "">("");
const error = ref("");
const calculating = ref(false);
const coverage = ref<{
  size: number;
  levels: number[];
  target_percent: number;
}>();

const materials = {
  partition: "轻质隔墙",
  brick: "砖墙",
  concrete: "混凝土墙",
};
const wallColors = {
  partition: "#897563",
  brick: "#7f503c",
  concrete: "#362d25",
};
const bands = ["#fbf7ef", "#d9987b", "#e9c795", "#a9bccd"];
const valid = computed(() =>
  [width.value, height.value].every((value) => value >= 1 && value <= 50),
);
const plan = computed<WifiPlan>(() => ({
  width_cm: Math.round(width.value * 100),
  height_cm: Math.round(height.value * 100),
  access_points: accessPoints.value.map((point) => ({ ...point })),
  walls: walls.value.map((wall) => ({ ...wall })),
}));
function geometry(plan: WifiPlan) {
  return JSON.stringify([
    plan.width_cm,
    plan.height_cm,
    plan.access_points.map(({ x, y }) => [x, y]),
    plan.walls.map(({ from, to, material }) => [
      from.x,
      from.y,
      to.x,
      to.y,
      material,
    ]),
  ]);
}

const changed = computed(
  () =>
    geometry(plan.value) !==
    geometry(
      props.saved ?? {
        width_cm: 1200,
        height_cm: 800,
        access_points: [],
        walls: [],
      },
    ),
);
const hint = computed(() =>
  mode.value === "ap"
    ? "点击网格放置 AP  ·  最多 8 个"
    : anchor.value
      ? "点击墙体终点"
      : "点击墙体起点，再点击终点  ·  最多 24 段",
);
const aspect = computed(() => (valid.value ? width.value / height.value : 1.5));

onMounted(() => dialog.value?.showModal());
watch(mode, () => {
  anchor.value = undefined;
});

function exit() {
  if (changed.value && !window.confirm("离开时放弃未保存的规划？")) return;
  dialog.value?.close();
  emit("close");
}

function mark(event: MouseEvent) {
  if (!valid.value || props.busy) return;
  const bounds = canvas.value?.getBoundingClientRect();
  if (!bounds) return;
  const grid = (offset: number, size: number) =>
    Math.min(1000, Math.max(0, Math.round((offset / size) * 40) * 25));
  const point = {
    x: grid(event.clientX - bounds.left, bounds.width),
    y: grid(event.clientY - bounds.top, bounds.height),
  };
  if (mode.value === "ap") {
    if (
      accessPoints.value.length < 8 &&
      !accessPoints.value.some(({ x, y }) => x === point.x && y === point.y)
    )
      accessPoints.value.push(point);
  } else if (!anchor.value) {
    if (walls.value.length < 24) anchor.value = point;
  } else if (point.x !== anchor.value.x || point.y !== anchor.value.y) {
    walls.value.push({
      from: anchor.value,
      to: point,
      material: material.value,
    });
    anchor.value = undefined;
  }
}

function draw() {
  const surface = canvas.value;
  const context = surface?.getContext("2d");
  if (!surface || !context) return;
  const { width: w, height: h } = surface;
  context.fillStyle = bands[0];
  context.fillRect(0, 0, w, h);
  if (coverage.value) {
    const { size, levels } = coverage.value;
    levels.forEach((level, index) => {
      context.fillStyle = bands[level];
      context.fillRect(
        Math.floor(((index % size) * w) / size),
        Math.floor((Math.floor(index / size) * h) / size),
        Math.ceil(w / size),
        Math.ceil(h / size),
      );
    });
  }
  context.strokeStyle = "#55403124";
  context.lineWidth = 1;
  context.beginPath();
  for (let i = 1; i < 10; i++) {
    context.moveTo((w * i) / 10, 0);
    context.lineTo((w * i) / 10, h);
    context.moveTo(0, (h * i) / 10);
    context.lineTo(w, (h * i) / 10);
  }
  context.stroke();
  for (const wall of walls.value) {
    context.strokeStyle = wallColors[wall.material];
    context.lineWidth = 5;
    context.lineCap = "round";
    context.beginPath();
    context.moveTo((wall.from.x * w) / 1000, (wall.from.y * h) / 1000);
    context.lineTo((wall.to.x * w) / 1000, (wall.to.y * h) / 1000);
    context.stroke();
  }
  if (anchor.value) {
    context.fillStyle = "#9c6436";
    context.fillRect(
      (anchor.value.x * w) / 1000 - 4,
      (anchor.value.y * h) / 1000 - 4,
      8,
      8,
    );
  }
  const radius = Math.min(14, Math.floor(Math.min(w, h) / 3));
  accessPoints.value.forEach((point, index) => {
    const x = Math.min(
      w - radius - 1,
      Math.max(radius + 1, (point.x * w) / 1000),
    );
    const y = Math.min(
      h - radius - 1,
      Math.max(radius + 1, (point.y * h) / 1000),
    );
    context.fillStyle = "#352820";
    context.beginPath();
    context.arc(x, y, radius, 0, Math.PI * 2);
    context.fill();
    context.fillStyle = "#fff7eb";
    context.font = `bold ${radius}px sans-serif`;
    context.textAlign = "center";
    context.textBaseline = "middle";
    context.fillText(String(index + 1), x, y);
  });
}

watch(
  [plan, valid],
  (_, __, onCleanup) => {
    let stale = false;
    error.value = "";
    coverage.value = undefined;
    calculating.value = valid.value && Boolean(accessPoints.value.length);
    const timer = setTimeout(async () => {
      if (!calculating.value) return;
      try {
        const response = await fetch("/api/wifi/coverage", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(plan.value),
        });
        if (!response.ok) throw new Error(await response.text());
        const result = await response.json();
        if (!stale) coverage.value = result;
      } catch (cause) {
        if (!stale) error.value = (cause as Error).message;
      } finally {
        if (!stale) calculating.value = false;
      }
    }, 140);
    onCleanup(() => {
      stale = true;
      clearTimeout(timer);
    });
  },
  { immediate: true },
);

watch([canvas, plan, coverage, anchor], () => void nextTick(draw), {
  flush: "post",
});
</script>

<template>
  <dialog
    ref="dialog"
    class="wifi-dialog"
    aria-labelledby="wifi-title"
    @cancel.prevent="exit"
  >
    <header class="wifi-head">
      <div>
        <h2 id="wifi-title">Wi-Fi 覆盖规划</h2>
        <p>5 GHz 模拟预估，需在安装后现场验证。</p>
      </div>
      <button
        type="button"
        aria-label="关闭规划"
        :disabled="busy"
        @click="exit"
      >
        关闭
      </button>
    </header>
    <div class="wifi-layout">
      <section class="wifi-sheet">
        <div class="wifi-dimensions">
          <label
            >户型宽度（米）<input
              v-model.number="width"
              type="number"
              min="1"
              max="50"
              step="0.1"
              :disabled="busy"
          /></label>
          <label
            >户型深度（米）<input
              v-model.number="height"
              type="number"
              min="1"
              max="50"
              step="0.1"
              :disabled="busy"
          /></label>
        </div>
        <div class="wifi-tools" aria-label="布点工具">
          <button
            type="button"
            :aria-pressed="mode === 'ap'"
            @click="mode = 'ap'"
          >
            放置 AP
          </button>
          <button
            type="button"
            :aria-pressed="mode === 'wall'"
            @click="mode = 'wall'"
          >
            绘制墙体
          </button>
          <select
            v-if="mode === 'wall'"
            v-model="material"
            aria-label="墙体材质"
          >
            <option v-for="(name, key) in materials" :key="key" :value="key">
              {{ name }}
            </option>
          </select>
        </div>
        <p class="wifi-hint">
          {{ valid ? hint : "请输入 1–50 米的宽度和深度" }}
        </p>
        <div class="wifi-map">
          <canvas
            ref="canvas"
            :width="Math.round(640 * Math.min(aspect / 1.5, 1))"
            :height="Math.round(640 * Math.min(1 / aspect, 0.67))"
            aria-label="Wi-Fi 点位和覆盖图"
            @click="mark"
          ></canvas>
        </div>
        <div class="wifi-legend">
          <span><i :style="{ background: bands[3] }"></i> ≥ −67 dBm</span>
          <span><i :style="{ background: bands[2] }"></i> −75 至 −67 dBm</span>
          <span><i :style="{ background: bands[1] }"></i> &lt; −75 dBm</span>
        </div>
      </section>
      <section class="wifi-details">
        <p class="wifi-metric">
          {{ coverage ? `${coverage.target_percent}%` : "—" }}
        </p>
        <p class="wifi-hint">
          {{ calculating ? "计算覆盖中…" : "预计达到 −67 dBm 的面积" }}
        </p>
        <p v-if="error || props.error" class="alert" role="alert">
          {{ error || props.error }}
        </p>
        <h3>
          AP 点位 <span>{{ accessPoints.length }}</span>
        </h3>
        <ul class="wifi-list">
          <li
            v-for="(point, index) in accessPoints"
            :key="`${point.x},${point.y}`"
          >
            AP {{ index + 1
            }}<button
              type="button"
              :disabled="busy"
              :aria-label="`移除 AP ${index + 1}`"
              @click="accessPoints.splice(index, 1)"
            >
              移除
            </button>
          </li>
        </ul>
        <h3>
          墙体 <span>{{ walls.length }}</span>
        </h3>
        <ul class="wifi-list">
          <li v-for="(wall, index) in walls" :key="index">
            {{ index + 1 }} · {{ materials[wall.material]
            }}<button
              type="button"
              :disabled="busy"
              :aria-label="`移除墙体 ${index + 1}`"
              @click="walls.splice(index, 1)"
            >
              移除
            </button>
          </li>
        </ul>
        <button
          class="primary"
          type="button"
          :disabled="!valid || busy || (!changed && Boolean(saved))"
          @click="emit('save', plan)"
        >
          保存规划
        </button>
        <div class="wifi-quote">
          <label
            >报价设备
            <select v-model="item" :disabled="!catalog.length || busy">
              <option value="">
                {{
                  catalog.length ? "从产品库选择 AP" : "请先在产品库添加设备"
                }}
              </option>
              <option
                v-for="entry in catalog"
                :key="entry.index"
                :value="entry.index"
              >
                {{ entry.name }} · {{ entry.price }}
              </option>
            </select>
          </label>
          <button
            type="button"
            :disabled="
              busy || changed || !saved || !accessPoints.length || item === ''
            "
            @click="item !== '' && emit('quote', item, accessPoints.length)"
          >
            添加 {{ accessPoints.length }} 个 AP 到报价
          </button>
        </div>
      </section>
    </div>
  </dialog>
</template>
