import { createApp } from "vue";
import App from "./App.vue";
import Questionnaire from "./Questionnaire.vue";
import "./style.css";

const shareKey = location.pathname.match(/^\/q\/([^/]+)\/?$/)?.[1];
const app = shareKey ? createApp(Questionnaire, { shareKey }) : createApp(App);
app.mount("#app");
