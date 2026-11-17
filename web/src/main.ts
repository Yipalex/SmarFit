import { createApp } from "vue";
import App from "./App.vue";
import Questionnaire from "./Questionnaire.vue";
import "./style.css";

try {
  if (localStorage.getItem("smarfit-theme") === "night") {
    document.documentElement.dataset.theme = "night";
  }
} catch {
  // Use the default theme when storage is unavailable.
}

const shareKey = location.pathname.match(/^\/q\/([^/]+)\/?$/)?.[1];
const app = shareKey ? createApp(Questionnaire, { shareKey }) : createApp(App);
app.mount("#app");
