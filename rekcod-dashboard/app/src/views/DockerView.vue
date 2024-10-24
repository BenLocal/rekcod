<template>
  <el-row class="tac">
    <el-col :span="12">
      <el-link type="primary" @click="$router.push(`/node`)"
        >Node: &nbsp;
      </el-link>
      {{ node_name }}
    </el-col>
  </el-row>
    <el-row>
        <el-tabs v-model="selected" type="border-card" closeable="false" @tab-change="on_tab_change" style="width: 100%;">
            <el-tab-pane v-for="item in tabItems" :key="item.name" :label="item.label" :name="item.name">
                <component :is="item.component" :node_name="node_name"></component>
            </el-tab-pane>
        </el-tabs>
    </el-row>
</template>

<script setup>
import { ref, shallowRef } from 'vue'
import DockerContainerList from '../components/docker/DockerContainerList.vue'
import DockerInfo from '../components/docker/DockerInfo.vue'

defineProps({ node_name: String })
const selected = ref("tab-1")
const tabItems = ref([
    {
        label: 'Containers',
        name: 'tab-1',
        component: shallowRef(DockerContainerList)
    },
    {
        label: 'Info',
        name: 'tab-2',
        component: shallowRef(DockerInfo)
    },
])

const on_tab_change = () => {
    console.log(selected.value)
}


</script>
<style scoped></style>
