// The engine's C functions, from crates/khmer-macos/src/lib.rs. Keep the two
// in step. Every char * handed out is UTF-8 and is given back with bk_free.
#include <stdbool.h>
#include <stdint.h>

typedef struct BKEngine BKEngine;

typedef struct {
    char *commit;
    char *preedit;
    bool handled;
} BKOutput;

void bk_warm_up(void);
BKEngine *bk_engine_new(void);
void bk_engine_free(BKEngine *engine);
BKOutput bk_on_key(BKEngine *engine, uint32_t key);
BKOutput bk_select(BKEngine *engine, uint32_t index);
BKOutput bk_commit(BKEngine *engine);
void bk_reset(BKEngine *engine);
bool bk_is_composing(BKEngine *engine);
char *bk_candidates(BKEngine *engine);
char *bk_typed(BKEngine *engine);
char *bk_take_corrections(BKEngine *engine);
uint32_t bk_set_user_words(const char *text);
char *bk_check_word(const char *line);
char *bk_convert(const char *input);
void bk_free(char *text);
void bk_output_free(BKOutput output);
