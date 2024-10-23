<template>
  <el-container>
    <el-header style="background-color: azure"></el-header>
    <el-container>
      <el-aside class="layout-el-aside">
        <el-menu
          popper-effect="light"
          :collapse="true"
          :router="true"
          :default-active="getDefaultActive()"
        >
          <el-menu-item
            v-for="menu in menus"
            :key="menu.path"
            :index="menu.path"
          >
            <el-icon><component :is="menu.icon" /></el-icon>
            <template #title>
              <span>{{ menu.title }}</span>
            </template>
          </el-menu-item>
        </el-menu>
      </el-aside>
      <el-main>
        <el-scrollbar>
          <RouterView />
        </el-scrollbar>
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup>
import { RouterView, useRouter } from 'vue-router'

const menus = [
  {
    path: '/dashboard',
    icon: 'Platform',
    title: '控制面板',
  },
  {
    path: '/node',
    icon: 'setting',
    title: '节点',
  },
]

function getDefaultActive() {
  const router = useRouter()
  const path = router.currentRoute.value.path

  for (const menu of menus) {
    if (path.startsWith(menu.path)) {
      return menu.path
    }
  }
  return '/'
}
</script>

<style scoped>
.layout-el-aside {
  width: auto !important;
}
</style>
