<template>
  <el-space wrap>
    <el-card
      shadow="hover"
      v-for="item in appList"
      :key="item.id"
      style="width: 300px; margin-bottom: 30px"
      @click="$router.push(`/app/${item.id}`)"
    >
      <template #header>{{ item.name }}</template>
      <p>{{ item.description }}</p>
      <template #footer> {{ item.version }}</template>
    </el-card>
  </el-space>
</template>

<script setup>
import { onMounted, ref } from 'vue'
import api from '../api'
import { ElMessage } from 'element-plus'

const appList = ref([])

const get_app_list = async () => {
  const { code, data, msg } = await (await api.getAppList()).data
  if (code !== 0) {
    ElMessage.error(msg || '获取Application列表失败')
    return
  }
  console.log(data)

  appList.value = data.map(item => {
    return {
      id: item.id,
      name: item.name,
      description: item.description,
      version: item.version,
    }
  })
}

onMounted(() => {
  get_app_list()
})
</script>

<style scoped>
.el-row {
  margin-bottom: 20px;
}
</style>
