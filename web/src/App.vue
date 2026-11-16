<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";

type Category = "equipment" | "service";
type Line = {
  name: string;
  category: Category;
  quantity: number;
  unit_cents: number;
};
type Quote = {
  lines: (Line & { amount_cents: number })[];
  total_cents: number;
};
type Project = {
  name: string;
  requirements: string;
  draft: Quote;
  published: Quote[];
};
type Workspace = { projects: Project[]; catalog: Line[] };
type Inquiry = {
  id: string;
  name: string;
  home: string;
  systems: string[];
  budget: string;
  notes: string;
};
type State = {
  workspace: Workspace;
  revision: number;
  questionnaire_key: string | null;
  inbox: Inquiry[];
};
type EditableLine = {
  key: number;
  name: string;
  category: Category;
  quantity: number;
  price: string;
};

const workspace = ref<Workspace>({ projects: [], catalog: [] });
const revision = ref(0);
const tab = ref<"projects" | "catalog" | "inbox">("projects");
const tabLabels = {
  projects: "项目与报价",
  catalog: "产品库",
  inbox: "需求收件箱",
};
const questionnaireKey = ref<string | null>(null);
const inbox = ref<Inquiry[]>([]);
const copied = ref(false);
const questionnaireURL = computed(() =>
  questionnaireKey.value
    ? new URL(`/q/${questionnaireKey.value}`, location.origin).href
    : "",
);
const selected = ref(0);
const historyVersion = ref<number | null>(null);
const lines = ref<EditableLine[]>([]);
const priced = ref<Quote>();
const priceError = ref("");
const error = ref("");
const message = ref("");
const ready = ref(false);
const busy = ref(false);
const version = ref<{
  name: string;
  marker: string;
  version: string;
  commit: string;
}>();
const projectForm = ref<HTMLDialogElement>();
const productForm = ref<HTMLDialogElement>();
const projectName = ref("");
const requirements = ref("");
const productName = ref("");
const productPrice = ref("");
const productCategory = ref<Category>("equipment");
const productIndex = ref<number | null>(null);
const printQuote = ref<{ name: string; version: number; quote: Quote }>();
let nextKey = 0;
let token = "";
let printRequest = 0;

const project = computed(() => workspace.value.projects[selected.value]);
const snapshot = computed(() =>
  historyVersion.value === null
    ? undefined
    : project.value?.published[historyVersion.value],
);
const visibleQuote = computed(() => snapshot.value ?? priced.value);
const currency = new Intl.NumberFormat("zh-CN", {
  style: "currency",
  currency: "CNY",
});
const money = (cents: number) => currency.format(cents / 100);
const categoryLabel = (category: Category) =>
  category === "equipment" ? "设备费" : "服务费";
const subtotal = (category: Category) =>
  (visibleQuote.value?.lines ?? [])
    .filter((line) => line.category === category)
    .reduce((total, line) => total + line.amount_cents, 0);

function cents(value: string) {
  const normalized = value.trim();
  if (!/^\d{1,7}(\.\d{1,2})?$/.test(normalized))
    throw new Error("请输入最多两位小数的单价");
  const [whole, fraction = ""] = normalized.split(".");
  return Number(whole) * 100 + Number(fraction.padEnd(2, "0"));
}

async function api<T>(path: string, body?: unknown): Promise<T> {
  const response = await fetch(path, {
    method: body === undefined ? "GET" : "POST",
    headers: {
      ...(path === "/api/workspace"
        ? { Authorization: `Bearer ${token}` }
        : {}),
      ...(body === undefined ? {} : { "Content-Type": "application/json" }),
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!response.ok) throw new Error(await response.text());
  return response.json();
}

function apply(state: State) {
  revision.value = state.revision;
  workspace.value = state.workspace;
  questionnaireKey.value = state.questionnaire_key;
  inbox.value = state.inbox ?? [];
}

function editable(line: Line): EditableLine {
  return {
    key: ++nextKey,
    name: line.name,
    category: line.category,
    quantity: line.quantity,
    price: (line.unit_cents / 100).toFixed(2),
  };
}

function showProject(index: number) {
  selected.value = index;
  historyVersion.value = null;
  tab.value = "projects";
  lines.value = (project.value?.draft.lines ?? []).map(editable);
  priced.value = project.value?.draft;
}

function draftLines(): Line[] {
  return lines.value.map((line) => ({
    name: line.name,
    category: line.category,
    quantity: line.quantity,
    unit_cents: cents(String(line.price)),
  }));
}

watch(
  lines,
  (_, __, onCleanup) => {
    priced.value = undefined;
    priceError.value = "";
    let canceled = false;
    const timer = setTimeout(async () => {
      try {
        const result = await api<Quote>("/api/price", draftLines());
        if (!canceled) priced.value = result;
      } catch (cause) {
        if (!canceled) priceError.value = (cause as Error).message;
      }
    }, 180);
    onCleanup(() => {
      canceled = true;
      clearTimeout(timer);
    });
  },
  { deep: true },
);

function randomKey() {
  return Array.from(crypto.getRandomValues(new Uint8Array(32)), (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
}

async function refreshInbox() {
  if (busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    apply(await api<State>("/api/workspace"));
  } catch (cause) {
    error.value = (cause as Error).message;
  } finally {
    busy.value = false;
  }
}

function openInbox() {
  tab.value = "inbox";
  message.value = "";
  if (ready.value) void refreshInbox();
}

async function copyQuestionnaire() {
  try {
    await navigator.clipboard.writeText(questionnaireURL.value);
    copied.value = true;
  } catch {
    error.value = "复制失败，请选中链接后手动复制。";
  }
}

function setQuestionnaire(open: boolean) {
  void mutate(
    { action: "set_questionnaire", key: open ? randomKey() : null },
    () => {
      copied.value = false;
      message.value = open ? "问卷链接已启用" : "问卷链接已关闭";
    },
  );
}

function resolveInquiry(id: string, createProject: boolean) {
  void mutate(
    { action: "resolve_inquiry", id, create_project: createProject },
    () => {
      if (createProject) showProject(workspace.value.projects.length - 1);
      message.value = createProject
        ? "需求已转为项目，可以开始报价"
        : "需求已移除";
    },
  );
}

async function load() {
  error.value = "";
  try {
    token = localStorage.getItem("smarfit-demo-key") ?? "";
    if (!/^[0-9a-f]{64}$/.test(token)) {
      token = randomKey();
      localStorage.setItem("smarfit-demo-key", token);
    }
    apply(await api<State>("/api/workspace"));
    showProject(
      Math.min(
        selected.value,
        Math.max(0, workspace.value.projects.length - 1),
      ),
    );
    ready.value = true;
  } catch (cause) {
    error.value = (cause as Error).message;
  }
}

onMounted(() => {
  void load();
  void api<NonNullable<typeof version.value>>("/api/version")
    .then((result) => {
      version.value = result;
    })
    .catch(() => {});
});

async function mutate(action: Record<string, unknown>, done: () => void) {
  if (busy.value) return;
  busy.value = true;
  error.value = "";
  message.value = "";
  try {
    apply(
      await api<State>("/api/workspace", {
        ...action,
        revision: revision.value,
      }),
    );
    done();
  } catch (cause) {
    error.value = (cause as Error).message;
  } finally {
    busy.value = false;
  }
}

function openProject() {
  error.value = "";
  projectName.value = "";
  requirements.value = "";
  projectForm.value?.showModal();
}

function createProject() {
  void mutate(
    {
      action: "create_project",
      name: projectName.value,
      requirements: requirements.value,
    },
    () => {
      showProject(workspace.value.projects.length - 1);
      projectForm.value?.close();
      message.value = "项目已创建";
    },
  );
}

function openProduct(index: number | null) {
  error.value = "";
  const item = index === null ? undefined : workspace.value.catalog[index];
  productIndex.value = index;
  productName.value = item?.name ?? "";
  productPrice.value = item ? (item.unit_cents / 100).toFixed(2) : "";
  productCategory.value = item?.category ?? "equipment";
  productForm.value?.showModal();
}

function saveProduct() {
  try {
    const item = {
      name: productName.value,
      category: productCategory.value,
      quantity: 1,
      unit_cents: cents(productPrice.value),
    };
    void mutate(
      { action: "save_product", index: productIndex.value, item },
      () => {
        productForm.value?.close();
        message.value = "产品库已更新";
      },
    );
  } catch (cause) {
    error.value = (cause as Error).message;
  }
}

function addFromCatalog(event: Event) {
  const select = event.target as HTMLSelectElement;
  const item = workspace.value.catalog[Number(select.value)];
  if (select.value !== "" && item) lines.value.push(editable(item));
  select.value = "";
}

function addService() {
  lines.value.push(
    editable({ name: "", category: "service", quantity: 1, unit_cents: 0 }),
  );
}

function saveQuote(publish: boolean) {
  try {
    void mutate(
      {
        action: "save_quote",
        project: selected.value,
        lines: draftLines(),
        publish,
      },
      () => {
        showProject(selected.value);
        if (publish) historyVersion.value = project.value.published.length - 1;
        message.value = publish ? "报价已发布，可以打印客户版" : "草稿已保存";
      },
    );
  } catch (cause) {
    error.value = (cause as Error).message;
  }
}

async function printCustomer() {
  if (!project.value || !snapshot.value || historyVersion.value === null)
    return;
  const request = ++printRequest;
  printQuote.value = {
    name: project.value.name,
    quote: snapshot.value,
    version: historyVersion.value + 1,
  };
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  if (request === printRequest) window.print();
}
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <a class="brand" href="/" aria-label="SmarFit 首页"
        ><span class="brand-mark">S</span
        ><span>SmarFit<small>智装工场</small></span></a
      >
      <nav aria-label="工作台导航">
        <button
          :class="{ active: tab === 'projects' }"
          @click="tab = 'projects'"
        >
          <span class="nav-icon" aria-hidden="true">▤</span> 项目与报价
        </button>
        <button :class="{ active: tab === 'catalog' }" @click="tab = 'catalog'">
          <span class="nav-icon" aria-hidden="true">◫</span> 产品库
        </button>
        <button :class="{ active: tab === 'inbox' }" @click="openInbox">
          <span class="nav-icon" aria-hidden="true">▱</span> 需求收件箱
        </button>
      </nav>
      <div class="sidebar-bottom">
        <span class="status-dot"></span> 试验工作台
      </div>
    </aside>

    <div class="main-shell">
      <header class="topbar">
        <span>工作台 <span class="slash">/</span> {{ tabLabels[tab] }}</span
        ><a
          href="https://github.com/Yipalex/SmarFit"
          target="_blank"
          rel="noopener noreferrer"
          >GitHub ↗</a
        >
      </header>
      <main>
        <div class="page-head">
          <div>
            <h1>{{ tabLabels[tab] }}</h1>
            <p>
              {{
                tab === "projects"
                  ? "记录需求、编制报价，保存每次对外发布的版本。"
                  : tab === "catalog"
                    ? "维护常用设备与服务，编制报价时直接选用。"
                    : "分享需求问卷，将收到的需求转为报价项目。"
              }}
            </p>
          </div>
          <button
            v-if="ready && tab !== 'inbox'"
            class="primary"
            :disabled="busy"
            @click="tab === 'projects' ? openProject() : openProduct(null)"
          >
            {{ tab === "projects" ? "＋ 新建项目" : "＋ 添加产品" }}
          </button>
        </div>
        <p class="demo-note">
          试验环境 · 工作台随当前浏览器保存，请使用虚构项目和产品资料。
        </p>
        <div v-if="error" class="alert" role="alert">
          {{ error }}
          <button
            type="button"
            @click="tab === 'inbox' ? refreshInbox() : load()"
          >
            重新加载
          </button>
        </div>
        <p v-if="message" class="success" role="status">{{ message }}</p>
        <p v-if="!ready && !error" role="status">正在加载工作台…</p>

        <div v-if="ready && tab === 'projects'" class="workspace-grid">
          <section class="panel projects">
            <div class="panel-heading">
              <h2>项目列表</h2>
              <span class="count">{{ workspace.projects.length }}</span>
            </div>
            <div v-if="workspace.projects.length" class="project-list">
              <button
                v-for="(entry, index) in workspace.projects"
                :key="index"
                :class="{ current: selected === index }"
                :disabled="busy"
                @click="showProject(index)"
              >
                <span class="project-icon" aria-hidden="true">⌂</span
                ><span
                  ><strong>{{ entry.name }}</strong
                  ><small>{{
                    entry.published.length
                      ? `已发布 ${entry.published.length} 版`
                      : "报价草稿"
                  }}</small></span
                >
              </button>
            </div>
            <p v-else class="empty-list">还没有项目</p>
          </section>

          <section v-if="project" class="panel editor" :aria-busy="busy">
            <div class="panel-heading editor-heading">
              <div>
                <h2>{{ project.name }}</h2>
                <p v-if="project.requirements" class="requirement">
                  {{ project.requirements }}
                </p>
              </div>
              <label class="version-field"
                >报价版本<select v-model="historyVersion" :disabled="busy">
                  <option :value="null">编辑草稿</option>
                  <option
                    v-for="(_, index) in project.published"
                    :key="index"
                    :value="index"
                  >
                    发布版 {{ index + 1 }}
                  </option>
                </select></label
              >
            </div>
            <div v-if="!snapshot" class="toolbar">
              <select
                aria-label="从产品库选品"
                :disabled="
                  busy || !workspace.catalog.length || lines.length >= 100
                "
                @change="addFromCatalog"
              >
                <option value="">
                  {{
                    workspace.catalog.length ? "从产品库选品" : "产品库暂无条目"
                  }}
                </option>
                <option
                  v-for="(item, index) in workspace.catalog"
                  :key="index"
                  :value="index"
                >
                  {{ item.name }} · {{ money(item.unit_cents) }}
                </option>
              </select>
              <button
                type="button"
                :disabled="busy || lines.length >= 100"
                @click="addService"
              >
                ＋ 添加服务项
              </button>
            </div>
            <div class="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th>项目名称</th>
                    <th>分类</th>
                    <th>数量</th>
                    <th>单价</th>
                    <th>小计</th>
                    <th v-if="!snapshot"><span class="sr-only">操作</span></th>
                  </tr>
                </thead>
                <tbody v-if="snapshot">
                  <tr v-for="(line, index) in snapshot.lines" :key="index">
                    <td>{{ line.name }}</td>
                    <td>{{ categoryLabel(line.category) }}</td>
                    <td>{{ line.quantity }}</td>
                    <td>{{ money(line.unit_cents) }}</td>
                    <td>{{ money(line.amount_cents) }}</td>
                  </tr>
                </tbody>
                <tbody v-else>
                  <tr v-for="(line, index) in lines" :key="line.key">
                    <td>
                      <input
                        v-model="line.name"
                        aria-label="项目名称"
                        placeholder="例如 安装调试"
                        maxlength="100"
                        :disabled="busy"
                      />
                    </td>
                    <td>
                      <select
                        v-model="line.category"
                        aria-label="分类"
                        :disabled="busy"
                      >
                        <option value="equipment">设备费</option>
                        <option value="service">服务费</option>
                      </select>
                    </td>
                    <td>
                      <input
                        v-model.number="line.quantity"
                        aria-label="数量"
                        class="number"
                        type="number"
                        min="1"
                        max="9999"
                        :disabled="busy"
                      />
                    </td>
                    <td>
                      <input
                        v-model="line.price"
                        aria-label="单价（元）"
                        class="price"
                        inputmode="decimal"
                        :disabled="busy"
                      />
                    </td>
                    <td class="amount">
                      {{
                        priced?.lines[index]
                          ? money(priced.lines[index].amount_cents)
                          : "—"
                      }}
                    </td>
                    <td>
                      <button
                        class="quiet"
                        :aria-label="'移除第 ' + (index + 1) + ' 项'"
                        :disabled="busy"
                        @click="lines.splice(index, 1)"
                      >
                        ×
                      </button>
                    </td>
                  </tr>
                  <tr v-if="!lines.length">
                    <td colspan="6" class="empty-cell">
                      从产品库选品，或直接添加服务项。
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p v-if="!snapshot && priceError" class="alert inline" role="alert">
              {{ priceError }}
            </p>
            <div class="summary">
              <span
                >设备
                {{ visibleQuote ? money(subtotal("equipment")) : "—" }}</span
              ><span
                >服务
                {{ visibleQuote ? money(subtotal("service")) : "—" }}</span
              ><strong>{{
                visibleQuote ? money(visibleQuote.total_cents) : "—"
              }}</strong>
            </div>
            <div class="actions">
              <button v-if="snapshot" class="primary" @click="printCustomer">
                打印 / 保存 PDF
              </button>
              <template v-else
                ><button :disabled="busy || !priced" @click="saveQuote(false)">
                  保存草稿</button
                ><button
                  class="primary"
                  :disabled="busy || !priced || !lines.length"
                  @click="saveQuote(true)"
                >
                  发布报价
                </button></template
              >
            </div>
          </section>
          <section v-else class="panel empty-workspace">
            <span aria-hidden="true">▤</span>
            <h2>开始第一份报价</h2>
            <p>先创建一个项目，填入客户需求，再添加设备和服务。</p>
            <button class="primary" @click="openProject">＋ 新建项目</button>
          </section>
        </div>

        <div v-if="ready && tab === 'inbox'" class="inbox-grid">
          <section class="panel">
            <div class="panel-heading"><h2>客户问卷链接</h2></div>
            <div class="inbox-content">
              <template v-if="questionnaireURL">
                <label class="survey-link"
                  >公开填写链接<input
                    :value="questionnaireURL"
                    readonly
                    @focus="($event.target as HTMLInputElement).select()"
                /></label>
                <div class="inbox-actions">
                  <button class="primary" @click="copyQuestionnaire">
                    {{ copied ? "已复制" : "复制链接" }}
                  </button>
                  <a
                    class="survey-preview"
                    :href="questionnaireURL"
                    target="_blank"
                    rel="noopener noreferrer"
                    >打开问卷 ↗</a
                  >
                  <button :disabled="busy" @click="setQuestionnaire(false)">
                    关闭链接
                  </button>
                </div>
              </template>
              <button
                v-else
                class="primary"
                :disabled="busy"
                @click="setQuestionnaire(true)"
              >
                创建问卷链接
              </button>
            </div>
          </section>
          <section class="panel">
            <div class="panel-heading">
              <h2>待处理需求</h2>
              <span class="count">{{ inbox.length }}</span
              ><button
                class="inbox-refresh"
                :disabled="busy"
                @click="refreshInbox"
              >
                刷新
              </button>
            </div>
            <p v-if="!inbox.length" class="empty-list">还没有收到需求。</p>
            <article
              v-for="entry in [...inbox].reverse()"
              :key="entry.id"
              class="inbox-entry"
            >
              <h3>{{ entry.name }}</h3>
              <p>
                {{ entry.home }} · {{ entry.systems.join("、") }} ·
                {{ entry.budget }}
              </p>
              <p v-if="entry.notes">{{ entry.notes }}</p>
              <div class="inbox-actions">
                <button
                  class="primary"
                  :disabled="busy"
                  @click="resolveInquiry(entry.id, true)"
                >
                  转为项目
                </button>
                <button
                  :disabled="busy"
                  @click="resolveInquiry(entry.id, false)"
                >
                  移除需求
                </button>
              </div>
            </article>
          </section>
        </div>

        <section v-if="ready && tab === 'catalog'" class="panel">
          <div class="panel-heading">
            <h2>常用产品与服务</h2>
            <span class="count">{{ workspace.catalog.length }}</span>
          </div>
          <div class="table-scroll">
            <table>
              <thead>
                <tr>
                  <th>名称</th>
                  <th>分类</th>
                  <th>对外单价</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="(item, index) in workspace.catalog" :key="index">
                  <td>{{ item.name }}</td>
                  <td>{{ categoryLabel(item.category) }}</td>
                  <td>{{ money(item.unit_cents) }}</td>
                  <td>
                    <button :disabled="busy" @click="openProduct(index)">
                      编辑
                    </button>
                  </td>
                </tr>
                <tr v-if="!workspace.catalog.length">
                  <td colspan="4" class="empty-cell">
                    添加常用设备或服务，之后可在任意项目中选用。
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>
      </main>
      <footer v-if="version">
        {{ version.name }} · {{ version.marker }} · v{{ version.version }} ·
        <code :title="version.commit">{{ version.commit.slice(0, 12) }}</code>
      </footer>
    </div>

    <dialog ref="projectForm">
      <form @submit.prevent="createProject">
        <h2>新建项目</h2>
        <label
          >项目名称<input
            v-model="projectName"
            required
            maxlength="80"
            placeholder="例如 松庭公寓（示例）"
            autofocus
        /></label>
        <label
          >客户需求<textarea
            v-model="requirements"
            maxlength="500"
            rows="4"
            placeholder="例如 客厅灯光、卧室电动窗帘"
          />
        </label>
        <p v-if="error" class="alert" role="alert">{{ error }}</p>
        <div class="actions">
          <button type="button" :disabled="busy" @click="projectForm?.close()">
            取消</button
          ><button class="primary" :disabled="busy">创建项目</button>
        </div>
      </form>
    </dialog>
    <dialog ref="productForm">
      <form @submit.prevent="saveProduct">
        <h2>{{ productIndex === null ? "添加产品" : "编辑产品" }}</h2>
        <label
          >名称<input v-model="productName" required maxlength="100" autofocus
        /></label>
        <label
          >分类<select v-model="productCategory">
            <option value="equipment">设备费</option>
            <option value="service">服务费</option>
          </select></label
        >
        <label
          >对外单价（元）<input
            v-model="productPrice"
            required
            inputmode="decimal"
            placeholder="0.00"
        /></label>
        <p v-if="error" class="alert" role="alert">{{ error }}</p>
        <div class="actions">
          <button type="button" :disabled="busy" @click="productForm?.close()">
            取消</button
          ><button class="primary" :disabled="busy">保存产品</button>
        </div>
      </form>
    </dialog>
  </div>

  <article v-if="printQuote" class="print-quote">
    <p>SmarFit · 智装工场</p>
    <h1>{{ printQuote.name }} · 报价单</h1>
    <p>报价版本 {{ printQuote.version }}</p>
    <table>
      <thead>
        <tr>
          <th>项目名称</th>
          <th>分类</th>
          <th>数量</th>
          <th>单价</th>
          <th>小计</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(line, index) in printQuote.quote.lines" :key="index">
          <td>{{ line.name }}</td>
          <td>{{ categoryLabel(line.category) }}</td>
          <td>{{ line.quantity }}</td>
          <td>{{ money(line.unit_cents) }}</td>
          <td>{{ money(line.amount_cents) }}</td>
        </tr>
      </tbody>
    </table>
    <p class="print-total">合计 {{ money(printQuote.quote.total_cents) }}</p>
  </article>
</template>
