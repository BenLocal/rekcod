<template>
  <el-row class="tac">
    <el-col :span="12">
      <el-link type="primary" @click="$router.push(`/node`)"
        >Node: &nbsp;
      </el-link>
      {{ node_name }}
    </el-col>
  </el-row>
</template>

<script setup>
defineProps({ node_name: String })
</script>
<style scoped></style>
