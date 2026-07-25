import { createRouter, createWebHashHistory } from "vue-router";
import MainPanel from "../views/MainPanel.vue";
import Favorites from "../views/Favorites.vue";
import Settings from "../views/Settings.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "main", component: MainPanel },
    { path: "/favorites", name: "favorites", component: Favorites },
    { path: "/settings", name: "settings", component: Settings },
  ],
});

export default router;
