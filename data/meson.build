# Configure Desktop file
desktop_conf = configuration_data()
desktop_conf.set('APP_ID', base_id)
desktop_conf.set('APP_NAME', app_name)
desktop_conf.set('ICON_NAME', base_id)

desktop_in = configure_file(
  input: 'io.github.tobagin.Sidestep.desktop.in',
  output: 'io.github.tobagin.Sidestep.configured.desktop.in',
  configuration: desktop_conf
)

desktop_file = i18n.merge_file(
  input: desktop_in,
  output: base_id + '.desktop',
  type: 'desktop',
  po_dir: '../po',
  install: true,
  install_dir: get_option('datadir') / 'applications'
)

desktop_utils = find_program('desktop-file-validate', required: false)
if desktop_utils.found()
  test('Validate desktop file', desktop_utils, args: [desktop_file])
endif

# Configure AppStream file
appstream_in = configure_file(
  input: 'io.github.tobagin.Sidestep.metainfo.xml.in',
  output: 'io.github.tobagin.Sidestep.configured.metainfo.xml.in',
  configuration: desktop_conf
)

appstream_file = i18n.merge_file(
  input: appstream_in,
  output: base_id + '.metainfo.xml',
  po_dir: '../po',
  install: true,
  install_dir: get_option('datadir') / 'metainfo'
)

appstreamcli = find_program('appstreamcli', required: false)
if appstreamcli.found()
  test('Validate appstream file', appstreamcli,
    args: ['validate', '--no-net', '--explain', appstream_file])
endif

# Configure GSchema
gschema_conf = configuration_data()
gschema_conf.set('APP_ID', base_id)
gschema_conf.set('APP_PATH', '/' + base_id.replace('.', '/') + '/')
configure_file(
  input: 'io.github.tobagin.Sidestep.gschema.xml.in',
  output: base_id + '.gschema.xml',
  configuration: gschema_conf,
  install: true,
  install_dir: get_option('datadir') / 'glib-2.0' / 'schemas'
)

compile_schemas = find_program('glib-compile-schemas', required: false)
if compile_schemas.found()
  test('Validate schema file',
    compile_schemas,
    args: ['--strict', '--dry-run', meson.current_source_dir()])
endif

# Compile Blueprint files
blueprintc = find_program('blueprint-compiler', required: true)

blueprint_sources = files(
  'ui/window.blp',
  'ui/terminal_overlay.blp',
  'ui/preferences_dialog.blp',
  'ui/pages/safety.blp',
  'ui/pages/unlocking.blp',
  'ui/pages/distro_selection.blp',
  'ui/pages/device_details.blp',
  'ui/pages/flashing.blp',
  'ui/pages/waiting.blp',
  'ui/pages/success.blp',
  'ui/pages/unsupported_device.blp',
  'ui/wizards/install_wizard.blp',
)

blueprint_ui = custom_target('blueprint-compile',
  input: blueprint_sources,
  output: '.',
  command: [
    blueprintc, 'batch-compile',
    '@OUTPUT@',
    '@CURRENT_SOURCE_DIR@',
    '@INPUT@',
  ],
)

gnome.compile_resources('sidestep',
  'resources.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: get_option('datadir') / 'sidestep',
  dependencies: blueprint_ui,
)

subdir('icons')
