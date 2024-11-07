import { createRouter, createWebHashHistory } from 'vue-router'
import BaseLayout from '../layout/BaseLayout.vue'

const router = createRouter({
  history: createWebHashHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      redirect: '/dashboard',
      component: BaseLayout,
      children: [
        {
          path: 'dashboard',
          name: 'dashboard',
          component: () => import('../views/DashboardView.vue'),
        },
        {
          path: 'node',
          name: 'node',
          component: () => import('../views/NodeManagerView.vue'),
        },
        {
          path: 'node/docker',
          name: 'docker',
          component: () => import('../views/DockerView.vue'),
          props: route => ({ node_name: route.query.node_name }),
        },
      ],
    },
  ],
})

export default router
