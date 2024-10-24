<template>
  <el-descriptions title="Docker" border>
    <el-descriptions-item label="容器总个数">{{ info.Containers }}</el-descriptions-item>
    <el-descriptions-item label="运行中容器个数">{{info.ContainersRunning}}</el-descriptions-item>
    <el-descriptions-item label="暂停容器个数">{{info.ContainersPaused}}</el-descriptions-item>
    <el-descriptions-item label="停止容器个数">{{info.ContainersStopped}}</el-descriptions-item>
    <el-descriptions-item label="镜像总个数">{{info.Images}}</el-descriptions-item>
    <el-descriptions-item label="版本">{{info.ServerVersion}}</el-descriptions-item>
  </el-descriptions>
</template>

<script setup>

import { onMounted, ref } from 'vue';
import api from '../../api';
import { ElMessage } from 'element-plus';

const props = defineProps({
    node_name: String
})

const info = ref({})

const refresh = async () => {
  await get_docker_info(props.node_name)
}

const get_docker_info = async (node_name) => {
    const { code, data, msg }  = await (await api.getDockerInfoByNode(node_name)).data
    if (code !== 0) {
        ElMessage.error(msg ? msg : '获取容器信息失败')
        return
    }

    info.value = data
}

onMounted(() => {
    refresh()
})

defineExpose({
    refresh
})

</script>

<style scoped></style>