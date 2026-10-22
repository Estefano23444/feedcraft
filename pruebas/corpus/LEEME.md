# Corpus de prueba

Copia aquí las fotos reales del corpus (sección 9 de la especificación): retrato, paisaje,
producto, noche, interior, fotos de móvil con EXIF rotado, una foto pequeña de 800 px y un
PNG con transparencia.

Esta carpeta está en `.gitignore`: las fotos **no se suben** al repositorio público.
Solo este archivo se versiona.

Las pruebas automáticas no dependen de estas fotos: usan imágenes sintéticas que genera
`npm run sinteticas` en `pruebas/sinteticas/`. Si hay fotos aquí, `cargo test` también
las pasa por el proceso completo (prueba `corpus_exporta`).
