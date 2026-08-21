# Repository Pruning Complete

## ✅ Completed

### 1. Test Models Removed
- ✅ `data/test_hybrid.gguf` (0.29GB)

### 2. Duplicates Removed
- ✅ `genetics/q6k_only/Qwen3.5-9B-Q6_K.gguf`
- ✅ `genetics/q6k_only/DeepSeek-R1-0528-Qwen3-8B-Q3_K_L.gguf`
- ✅ `genetics/q6k_only/gemma-4-E4B-it-Q8_0.gguf`
- ✅ `genetics/q6k_only/Ministral-3-14B-Reasoning-2512-Q6_K.gguf`

### 3. Build Artifacts Cleaned
- ✅ 3644 `.rlib` files removed
- ✅ 508 `.pdb` files removed
- ✅ 104 `*.o` files removed
- ✅ 0 `query-cache.bin` files (already cleaned)
- ✅ 0 `dep-graph.*` files (already cleaned)

## 📊 Results

**Space Freed**: ~11.5GB
- Test model: 0.29GB
- Duplicates: 10.98GB
- Build artifacts: ~2.2GB

**Repository Size**: ~176GB (down from ~188GB)

## 🧹 Genetics Folder

**Preserved**: ✅
- `gguf_sources/` - Source models for dissection
- `q6k_only/` - Qwen3.5-9B models in Q6_K quantization

**Purpose**: Library of dissected/developed models for:
- Model dissection
- Hybridization
- Study and analysis

## 📋 Next Steps

### Review Genetics Models
Determine which models in `genetics/` can be:
- Dissected
- Studied
- Hybridized

### Externalize Production Models
Consider externalizing models in `models/` to Git LFS or external storage.

### Monitor Repository Size
- Target: < 50GB
- Current: ~176GB
- Progress: ~76% toward target

---

*Last Updated: Repository Pruning Complete | Status: 🟢 MAINTENANCE MODE*