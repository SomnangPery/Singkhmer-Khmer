# Third-party notices

This project includes data and lexicons from other open-source projects. Each is listed here with its attribution, description, and licence.

---

## 1. Chhunneng/khmer-text-transliteration
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/01_chhunneng.txt`), 27,871 parallel pairs.
- **Source**: <https://github.com/Chhunneng/khmer-text-transliteration>
- **Author**: Chrea Chanchhunneng
- **Licence**: MIT License

```text
MIT License
Copyright (c) 2025 Chrea Chanchhunneng
```

---

## 2. Akkhara Keyboard & Desktop (Muyleang Ing)
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/08_akkhara_keyboard.txt`, 167,951 pairs), bigram transition weights in `crates/khmer-core/data/pairs.tsv` (14,735 pairs from `khmer_bigram.tsv.gz`), `datasets/02_akkhara_lexicon.txt`, `datasets/03_akkhara_abbrev.txt`, and benchmark sentences (`datasets/all_gathered_sentences.tsv`).
- **Sources**: 
  - <https://github.com/MuyleangIng/akkhara-keyboard>
  - <https://github.com/MuyleangIng/akkhara-desktop>
  - <https://huggingface.co/datasets/muyleanging/Akkhara-Sing-Khmer>
  - <https://huggingface.co/spaces/muyleanging/Akkhara-Demo>
- **Author**: Muyleang Ing
- **Licence**: Apache License 2.0 / Creative Commons Attribution-ShareAlike 4.0 (CC BY-SA 4.0)

---

## 3. Richard Cheam Roman2Khmer
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/09_cheam_suite.txt`, 104,285 pairs across `spell_train.tsv`, `llm_spellings.tsv`, `mined_pairs.tsv`, `loanwords.tsv`) and 897 chat test sentences in `datasets/all_gathered_sentences.tsv`.
- **Source**: <https://github.com/richardcheam/roman2khmer>
- **Author**: Richard Cheam
- **Licence**: MIT License

---

## 4. Typer (Nel Sokchhunly)
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/10_typer_dataset.txt`, 38,189 pairs across `mobile-keyboard-data.json`, `old-dataset.txt`, `hand-romans.txt`, `user-romans.txt`) and corpus frequencies in `datasets/merged_frequencies.tsv`.
- **Source**: <https://github.com/Nel-sokchhunly/typer>
- **Author**: Nel Sokchhunly
- **Licence**: MIT License

---

## 5. KhmerIME (Darh Noel)
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/11_khmerime.txt`, 75,330 pairs across `roman_lookup.csv` and `chunk_tonle.csv`).
- **Source**: <https://github.com/darhnoel/khmerime>
- **Author**: Darh Noel
- **Licence**: MIT License

---

## 6. SingKhmerKeyboard / XingKhmer (Heng Huy Long)
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/12_xingkhmer.txt`, 42,361 pairs from `xingkhmer.dict.yaml`).
- **Source**: <https://github.com/HengHuyLong/SingKhmerKeyboard>
- **Author**: Heng Huy Long
- **Licence**: MIT License

---

## 7. Khmer-Loeun (Chhay Sophal)
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/13_loeun_provinces_and_dict.txt`, 1,050 pairs across provinces, offices, relationships, schools, and dictionaries `a.json` to `z.json`).
- **Source**: <https://github.com/chhay-sophal/Khmer-Loeun>
- **Author**: Chhay Sophal
- **Licence**: MIT License

---

## 8. SingKhmerTran (Sahaka)
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/14_singkhmertran_slang.txt`, chat slang abbreviations).
- **Source**: <https://github.com/chsahaka/singkhmertran>
- **Author**: Sahaka
- **Licence**: MIT License

---

## 9. M0nGKol Singkhmer
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/15_mongkol_and_pheasa.txt`, chat seed words and lexicons).
- **Source**: <https://github.com/M0nGKol/singkhmer>
- **Author**: Mongkol
- **Licence**: MIT License

---

## 10. PheasaKhmer Engine
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/15_mongkol_and_pheasa.txt`, chat spellings) and 285 native chat sentences in `datasets/all_gathered_sentences.tsv`.
- **Source**: <https://github.com/pheasaKhmer/khmer-engine>
- **Licence**: MIT License

---

## 11. Google Language Resources (Khmer Lexicon)
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/07_google_km_pairs.txt`, 62,093 phonetic spellings for 62,089 words).
- **Source**: <https://github.com/google/language-resources> (`km/data/lexicon.tsv`)
- **Author**: Google LLC
- **Licence**: Creative Commons Attribution 4.0 International (CC BY 4.0)

---

## 12. Leangsuor Latin-to-Khmer Transliteration
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/04_leangsuor_pairs.txt`, 2,922 spellings) and benchmark sentences.
- **Source**: <https://huggingface.co/leangsuor/latin_to_khmer_transliteration_model>
- **Author**: Leangsuor Kim
- **Licence**: Open Source

---

## 13. Kaikki.org / Wiktionary Khmer
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/05_kaikki_wiktionary.txt`, 7,706 spellings).
- **Source**: <https://kaikki.org/dictionary/Khmer/>
- **Licence**: Creative Commons Attribution-ShareAlike 3.0 Unported (CC BY-SA 3.0) and GFDL

---

## 14. Vuizur Wiktionary Dictionaries (Khmer-English)
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/16_vuizur_wiktionary.txt`, 9,225 pairs for 8,829 words).
- **Source**: <https://github.com/Vuizur/Wiktionary-Dictionaries> (`Khmer-English Wiktionary dictionary.tsv`)
- **Licence**: Creative Commons Attribution-ShareAlike 3.0 Unported (CC BY-SA 3.0) and GFDL

---

## 15. Sungkhum TipTap Khmer Multiple Spellings & Variants
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/18_spelling_variants_cross.txt`, 17,508 propagated variant pairs for 3,148 words).
- **Source**: <https://github.com/sungkhum/tiptap-khmer-line-breaker> (`public/dictionaries/khmer-multiple-spellings.txt`)
- **Author**: Sungkhum
- **Licence**: MIT License

---

## 16. Wikidata Cambodia & World Entities
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/06_wikidata_cambodia.txt` and `datasets/17_geographic_and_entities.txt`, 3,967 proper noun pairs).
- **Source**: Wikidata SPARQL Query Service (Cambodian administrative territories, entities, world nations, and capitals)
- **Licence**: Creative Commons Public Domain Dedication (CC0 1.0)

---

## 17. SEALang / SIL NRSI Khmer Frequency List
- **Used for**: Word frequency ranking in `datasets/merged_frequencies.tsv`.
- **Source**: <https://github.com/silnrsi/khmerlbdict> (`src/seafreq.txt`)
- **Author**: SIL International / SEALang Library
- **Licence**: MIT License

---

## 18. 5oni7a Khmer Profanity Dataset & Doeun Sentiment Engine (Facebook Corpora)
- **Used for**: Conversational word frequency ranking in `datasets/merged_frequencies.tsv` and social chat slang extraction (25,668 Facebook comments + reviews).
- **Sources**: 
  - <https://github.com/5oni7a/Khmer-Profanity>
  - <https://github.com/doeunbunheng/khmer_sentiment_engine>
- **Licence**: Open Source / Research

---

## 19. Cambodian Social Chat & Online Media Slang Suite
- **Used for**: `crates/khmer-core/data/words-imported.tsv` (`datasets/20_social_chat_pairs.txt`, 328 high-frequency social particles, youth abbreviations, and loanwords from Facebook, TikTok, and Telegram).
- **Licence**: MIT License

