<script setup lang="ts">
import { nextTick, ref } from "vue";

type Theme = "day" | "night";
const root = document.documentElement;
const theme = ref<Theme>(root.dataset.theme === "night" ? "night" : "day");
let switching = false;

async function changeTheme(value: Theme, event: MouseEvent) {
  if (value === theme.value || switching) return;
  function apply() {
    theme.value = value;
    root.dataset.theme = value;
    try {
      localStorage.setItem("smarfit-theme", value);
    } catch {
      // The theme still applies for the current page.
    }
    return nextTick();
  }
  if (
    !document.startViewTransition ||
    matchMedia("(prefers-reduced-motion: reduce)").matches
  ) {
    await apply();
    return;
  }

  const button = (
    event.currentTarget as HTMLButtonElement
  ).getBoundingClientRect();
  const x = button.x + button.width / 2;
  const y = button.y + button.height / 2;
  const radius = Math.hypot(
    Math.max(x, innerWidth - x),
    Math.max(y, innerHeight - y),
  );
  root.style.setProperty("--theme-origin", `${x}px ${y}px`);
  root.style.setProperty("--theme-radius", `${radius}px`);
  switching = true;
  root.dataset.themeTransition = "";
  try {
    await document.startViewTransition(apply).finished;
  } finally {
    switching = false;
    delete root.dataset.themeTransition;
    root.style.removeProperty("--theme-origin");
    root.style.removeProperty("--theme-radius");
  }
}
</script>

<template>
  <div class="theme-switch" role="group" aria-label="页面主题">
    <button
      type="button"
      :aria-pressed="theme === 'day'"
      @click="changeTheme('day', $event)"
    >
      <span aria-hidden="true">☼</span> 白天
    </button>
    <button
      type="button"
      :aria-pressed="theme === 'night'"
      @click="changeTheme('night', $event)"
    >
      <span aria-hidden="true">☾</span> 黑夜
    </button>
  </div>
</template>
