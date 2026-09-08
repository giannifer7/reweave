project('reweave-build-integration', version : '0.1.0')

reweave_exe = find_program('reweave', required : true)

gen_src = custom_target('gen-src',
  output : ['gen_src.stamp'],
  depfile : 'gen_src.d',
  command : [
    find_program('python3'),
    meson.current_source_dir() / 'scripts' / 'reweave_gen.py',
    meson.current_source_dir(),
    '@OUTPUT0@', '@DEPFILE@',
    reweave_exe,
  ],
  build_by_default : true,
)
