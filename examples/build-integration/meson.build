project('reweave-build-integration', version : '0.1.0')

reweave_exe = find_program('reweave', required : true)

gen_src = custom_target('gen-src',
  output : ['gen_src.stamp'],
  depfile : 'gen_src.d',
  command : [
    reweave_exe,
    '--dir', meson.current_source_dir() / 'src',
    '--ext', 'md',
    '-I', meson.current_source_dir(),
    '-o', meson.current_source_dir() / 'src',
    '--stamp', '@OUTPUT0@',
    '--depfile', '@DEPFILE@',
  ],
  build_by_default : true,
)
