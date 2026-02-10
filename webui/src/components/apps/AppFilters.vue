<template>
  <div class="page-header">
    <h2 class="page-title">{{ t('apps.title') }}</h2>
    <el-input
      v-model="searchModel"
      :placeholder="t('apps.search_placeholder')"
      clearable
      class="search-input"
    >
      <template #prefix>
        <el-icon><Search /></el-icon>
      </template>
    </el-input>
  </div>

  <div class="filter-tabs">
    <button :class="['filter-tab', { active: filterType === 'all' }]" @click="setFilter('all')">
      {{ t('apps.tabs.all') }} <span v-if="!loading">({{ totalCount }})</span>
    </button>
    <button
      :class="['filter-tab', { active: filterType === 'configured' }]"
      @click="setFilter('configured')"
    >
      {{ t('apps.tabs.configured') }} ({{ configuredCount }})
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Search } from '@element-plus/icons-vue'
import { useI18n } from '../../utils/i18n'

type FilterType = 'all' | 'configured'

const props = defineProps<{
  searchQuery: string
  filterType: FilterType
  totalCount: number
  configuredCount: number
  loading?: boolean
}>()

const emit = defineEmits<{ 'update:searchQuery': [string]; 'update:filterType': [FilterType] }>()

const { t } = useI18n()

const searchModel = computed({
  get: () => props.searchQuery,
  set: (val: string) => emit('update:searchQuery', val),
})

const setFilter = (type: FilterType) => emit('update:filterType', type)
</script>

<style scoped>
.page-header {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 1rem;
  margin-bottom: 0.5rem;
}

.page-title {
  font-size: 1.5rem;
  font-weight: 600;
  color: var(--text);
}

.search-input {
  width: 100%;
  max-width: 400px;
}

.filter-tabs {
  display: flex;
  gap: 0.5rem;
  padding-bottom: 0.5rem;
}

.filter-tab {
  flex: 1;
  padding: 0.5rem 1rem;
  background: var(--card-bg);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  color: var(--text-secondary);
  font-size: 0.875rem;
  font-weight: 500;
  transition: all 0.2s ease;
  white-space: nowrap;
  text-align: center;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
  -webkit-user-select: none;
}

.filter-tab:active {
  transform: scale(0.95);
  opacity: 0.8;
}

.filter-tab.active {
  background: var(--primary);
  color: white;
  border-color: var(--primary);
}
</style>
