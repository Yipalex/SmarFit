<script setup lang="ts">
import { onMounted, ref } from "vue";

const props = defineProps<{ shareKey: string }>();
const stages = ["新房待装修", "装修施工中", "已入住改造"];
const categories = ["灯光", "窗帘", "网络", "安防", "影音", "暖通"];
const budgets = [
  "待确认",
  "1 万元以下",
  "1–3 万元",
  "3–10 万元",
  "10 万元以上",
];
const name = ref("");
const home = ref("");
const systems = ref<string[]>([]);
const budget = ref("待确认");
const notes = ref("");
const error = ref("");
const open = ref(false);
const loading = ref(true);
const busy = ref(false);
const received = ref(false);
const id = crypto.randomUUID();
const endpoint = `/api/questionnaires/${encodeURIComponent(props.shareKey)}`;

onMounted(async () => {
  try {
    const response = await fetch(endpoint);
    if (!response.ok) throw new Error(await response.text());
    open.value = true;
  } catch (cause) {
    error.value = (cause as Error).message;
  } finally {
    loading.value = false;
  }
});

async function submit() {
  if (busy.value || received.value) return;
  error.value = "";
  if (!systems.value.length) {
    error.value = "请选择至少一项想了解的系统。";
    return;
  }
  busy.value = true;
  try {
    const response = await fetch(endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        id,
        name: name.value,
        home: home.value,
        systems: systems.value,
        budget: budget.value,
        notes: notes.value,
      }),
    });
    if (!response.ok) throw new Error(await response.text());
    received.value = true;
  } catch (cause) {
    error.value = (cause as Error).message;
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <main class="questionnaire">
    <header class="survey-brand">
      <span class="brand-mark" aria-hidden="true">S</span
      ><span>SmarFit · 智装工场</span>
    </header>
    <section class="panel survey-panel">
      <h1>智能家居需求问卷</h1>
      <p v-if="loading" role="status">正在打开问卷…</p>
      <p v-else-if="received" class="success" role="status">
        需求已提交，服务商可以在工作台查看。
      </p>
      <form v-else-if="open" class="survey-form" @submit.prevent="submit">
        <p class="demo-note">
          当前为试验问卷，请使用虚构资料，避免填写电话、姓名或详细地址。
        </p>
        <label
          >项目称呼<input
            v-model="name"
            required
            maxlength="80"
            placeholder="例如 松庭公寓（虚构）"
        /></label>
        <label
          >目前处于哪个阶段？
          <select v-model="home" required>
            <option disabled value="">请选择</option>
            <option v-for="stage in stages" :key="stage">{{ stage }}</option>
          </select>
        </label>
        <fieldset>
          <legend>想了解哪些智能家居系统？</legend>
          <div class="survey-options">
            <label v-for="category in categories" :key="category"
              ><input v-model="systems" type="checkbox" :value="category" />{{
                category
              }}</label
            >
          </div>
        </fieldset>
        <label
          >预算范围
          <select v-model="budget">
            <option v-for="range in budgets" :key="range">{{ range }}</option>
          </select>
        </label>
        <label
          >补充需求（选填）<textarea
            v-model="notes"
            maxlength="250"
            rows="3"
            placeholder="例如 卧室窗帘能定时开启"
          />
        </label>
        <p v-if="error" class="alert" role="alert">{{ error }}</p>
        <button class="primary" :disabled="busy">
          {{ busy ? "提交中…" : "提交需求" }}
        </button>
      </form>
      <p v-else class="alert" role="alert">{{ error }}</p>
    </section>
  </main>
</template>
