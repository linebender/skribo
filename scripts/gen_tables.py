import os.path
import sys

def gen_decomp(unicode_data_fn):
    decomps = []
    for line in open(unicode_data_fn):
        s = line.split(';')
        cp = int(s[0], 16)
        decomp = s[5]
        if decomp and not decomp.startswith('<'):
            decomp = [int(x, 16) for x in decomp.split()]
            if len(decomp) == 1 and decomp[0] == cp:
                print('codepoint {:x} maps to itself')
            if len(decomp) == 1: decomp.append(0)
            decomps.append((cp, decomp))
    print('pub const CANONICAL_DECOMP_KEY: [u32; {}] = ['.format(len(decomps)))
    for (cp, decomp) in decomps:
        print('    0x{:x},'.format(cp))
    print('];')
    print('pub const CANONICAL_DECOMP_VAL: [(u32, u32); {}] = ['.format(len(decomps)))
    for (cp, decomp) in decomps:
        print('    (0x{:x}, 0x{:x}),'.format(decomp[0], decomp[1]))
    print('];')

# This list was adapted from harfbuzz_sys/lib.rs (as of 0.3.1). It will probably need to
# be updated for future Harfbuzz releases, as it's missing some scripts in Unicode 12.
hb_scripts = set((
    'COMMON',
    'INHERITED',
    'UNKNOWN',
    'ARABIC',
    'ARMENIAN',
    'BENGALI',
    'CYRILLIC',
    'DEVANAGARI',
    'GEORGIAN',
    'GREEK',
    'GUJARATI',
    'GURMUKHI',
    'HANGUL',
    'HAN',
    'HEBREW',
    'HIRAGANA',
    'KANNADA',
    'KATAKANA',
    'LAO',
    'LATIN',
    'MALAYALAM',
    'ORIYA',
    'TAMIL',
    'TELUGU',
    'THAI',
    'TIBETAN',
    'BOPOMOFO',
    'BRAILLE',
    'CANADIAN_SYLLABICS',
    'CHEROKEE',
    'ETHIOPIC',
    'KHMER',
    'MONGOLIAN',
    'MYANMAR',
    'OGHAM',
    'RUNIC',
    'SINHALA',
    'SYRIAC',
    'THAANA',
    'YI',
    'DESERET',
    'GOTHIC',
    'OLD_ITALIC',
    'BUHID',
    'HANUNOO',
    'TAGALOG',
    'TAGBANWA',
    'CYPRIOT',
    'LIMBU',
    'LINEAR_B',
    'OSMANYA',
    'SHAVIAN',
    'TAI_LE',
    'UGARITIC',
    'BUGINESE',
    'COPTIC',
    'GLAGOLITIC',
    'KHAROSHTHI',
    'NEW_TAI_LUE',
    'OLD_PERSIAN',
    'SYLOTI_NAGRI',
    'TIFINAGH',
    'BALINESE',
    'CUNEIFORM',
    'NKO',
    'PHAGS_PA',
    'PHOENICIAN',
    'CARIAN',
    'CHAM',
    'KAYAH_LI',
    'LEPCHA',
    'LYCIAN',
    'LYDIAN',
    'OL_CHIKI',
    'REJANG',
    'SAURASHTRA',
    'SUNDANESE',
    'VAI',
    'AVESTAN',
    'BAMUM',
    'EGYPTIAN_HIEROGLYPHS',
    'IMPERIAL_ARAMAIC',
    'INSCRIPTIONAL_PAHLAVI',
    'INSCRIPTIONAL_PARTHIAN',
    'JAVANESE',
    'KAITHI',
    'LISU',
    'MEETEI_MAYEK',
    'OLD_SOUTH_ARABIAN',
    'OLD_TURKIC',
    'SAMARITAN',
    'TAI_THAM',
    'TAI_VIET',
    'BATAK',
    'BRAHMI',
    'MANDAIC',
    'CHAKMA',
    'MEROITIC_CURSIVE',
    'MEROITIC_HIEROGLYPHS',
    'MIAO',
    'SHARADA',
    'SORA_SOMPENG',
    'TAKRI',
    'BASSA_VAH',
    'CAUCASIAN_ALBANIAN',
    'DUPLOYAN',
    'ELBASAN',
    'GRANTHA',
    'KHOJKI',
    'KHUDAWADI',
    'LINEAR_A',
    'MAHAJANI',
    'MANICHAEAN',
    'MENDE_KIKAKUI',
    'MODI',
    'MRO',
    'NABATAEAN',
    'OLD_NORTH_ARABIAN',
    'OLD_PERMIC',
    'PAHAWH_HMONG',
    'PALMYRENE',
    'PAU_CIN_HAU',
    'PSALTER_PAHLAVI',
    'SIDDHAM',
    'TIRHUTA',
    'WARANG_CITI',
    'AHOM',
    'ANATOLIAN_HIEROGLYPHS',
    'HATRAN',
    'MULTANI',
    'OLD_HUNGARIAN',
    'SIGNWRITING',
    'ADLAM',
    'BHAIKSUKI',
    'MARCHEN',
    'OSAGE',
    'TANGUT',
    'NEWA',
    'MASARAM_GONDI',
    'NUSHU',
    'SOYOMBO',
    'ZANABAZAR_SQUARE',
    'DOGRA',
    'GUNJALA_GONDI',
    'HANIFI_ROHINGYA',
    'MAKASAR',
    'MEDEFAIDRIN',
    'OLD_SOGDIAN',
    'SOGDIAN',
))

def gen_script(script_data_fn):
    scripts = []
    for line in open(script_data_fn):
        line = line.rstrip()
        if line.startswith('#') or line == '':
            continue
        s = line.split(';')
        cp_range = s[0].rstrip().split('..')
        cp_start = int(cp_range[0], 16)
        if len(cp_range) == 2:
            cp_end = int(cp_range[1], 16) + 1
        else:
            cp_end = cp_start + 1
        script = s[1].split('#')[0].strip()
        if len(scripts) and scripts[-1][1] == cp_start:
            scripts[-1][1] = cp_end
        else:
            scripts.append([cp_start, cp_end, script])
    scripts.sort()
    warned_scripts = set()
    hb_data = []
    for (cp_start, cp_end, script) in scripts:
        hb_script_name = script.upper()
        if hb_script_name == 'CANADIAN_ABORIGINAL':
            hb_script_name = 'CANADIAN_SYLLABICS'
        if not hb_script_name in hb_scripts:
            if script in ('Elymaic', 'Nandinagari', 'Nyiakeng_Puachue_Hmong', 'Wancho'):
                if script not in warned_scripts:
                    print('// Warning: script {} not known by HarfBuzz'.format(script))
                    warned_scripts.add(script)
                continue
            print("Unknown script {}".format(script), file = sys.stderr)
            exit(1)
        hb_data.append((cp_start, cp_end, script, hb_script_name))
    print('pub const SCRIPT_KEY: [(u32, u32); {}] = ['.format(len(hb_data)))
    for (cp_start, cp_end, script, hb_script_name) in hb_data:
        print('    (0x{:x}, 0x{:x}), // {}'.format(cp_start, cp_end, script))
    print('];')
    print('pub const SCRIPT_VAL: [hb_script_t; {}] = ['.format(len(hb_data)))
    for (cp_start, cp_end, script, hb_script_name) in hb_data:
        print('    HB_SCRIPT_{},'.format(hb_script_name))
    print('];')

def gen_mirroring(mirror_data_fn):
    mirrors = []
    for line in open(mirror_data_fn):
        line = line.split('#')[0]
        line = line.rstrip()
        if line == '':
            continue
        fr, to = [int(cp.strip(), 16) for cp in line.split(';')]
        mirrors.append((fr, to))
    print('pub const MIRROR_KEY: [u32; {}] = ['.format(len(mirrors)))
    for (fr, to) in mirrors:
        print('    0x{:x}, // -> 0x{:x}'.format(fr, to))
    print('];')
    print('pub const MIRROR_VAL: [u32; {}] = ['.format(len(mirrors)))
    for (fr, to) in mirrors:
        print('    0x{:x}, // <- 0x{:x}'.format(to, fr))
    print('];')

def gen_copyright(readme_fn):
    for line in open(readme_fn):
        line = line.rstrip()
        if line == '#': return
        print(line.replace('#', '//'))

def main(args):
    ucd_dir = sys.argv[1]
    print("""// This file was automatically generated by gen_tables.py.
// It is derived from the Unicode Character Database; https://unicode.org/ucd/

// Those files contain the following copyright notice:
""")
    gen_copyright(os.path.join(ucd_dir, 'Readme.txt'))
    print('')
    print('use harfbuzz::sys::*;')
    print('')
    gen_decomp(os.path.join(ucd_dir, 'UnicodeData.txt'))
    gen_script(os.path.join(ucd_dir, 'Scripts.txt'))
    gen_mirroring(os.path.join(ucd_dir, 'BidiMirroring.txt'))

main(sys.argv)
