# Generate config.rs from template
conf = configuration_data()
conf.set_quoted('APP_ID', base_id)
conf.set_quoted('APP_NAME', app_name)
conf.set_quoted('VERSION', meson.project_version())
conf.set_quoted('GETTEXT_PACKAGE', 'sidestep')
conf.set_quoted('LOCALEDIR', get_option('prefix') / get_option('localedir'))
conf.set_quoted('PKGDATADIR', get_option('prefix') / get_option('datadir') / 'sidestep')
conf.set_quoted('SIDESTEP_DATA_DIR', get_option('prefix') / get_option('datadir') / 'sidestep')

configure_file(
  input: 'config.rs.in',
  output: 'config.rs',
  configuration: conf,
)

# Copy config.rs to source dir for cargo to find it
run_command('cp', meson.current_build_dir() / 'config.rs', meson.current_source_dir() / 'config.rs', check: false)
