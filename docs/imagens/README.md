# Imagens da documentação

Prints e banner usados no [README](../../README.md) e em [Como o Boreal funciona](../como-funciona.md). O repositório é público, então **todo print sai de um banco de demonstração**, nunca de dados reais.

## Como refazer os prints

1. Crie um banco novo com o [seed](../desenvolvimento/seed.md). As datas são relativas ao dia em que ele roda.
2. Feche o aplicativo e guarde `boreal.db`, `boreal.db-wal`, `boreal.db-shm` e a pasta `backups` de `%APPDATA%\br.com.boreal.pms`. O aplicativo só abre esse caminho, e parte dos dados pode estar só no `-wal`.
3. Coloque o banco de demonstração no lugar e abra o aplicativo com a depuração do WebView2 ligada:

   ```bash
   WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222" npm run tauri dev
   ```

4. Conecte-se à porta 9222 pelo protocolo do Chrome DevTools e use `Page.captureScreenshot`. Os prints atuais usam janela de 1600 × 1000 com escala 1,25, sem barra de rolagem. O aplicativo usa rotas com `#` (por exemplo, `#/mapa`).
5. Feche o aplicativo e devolva os arquivos guardados.

## Antes de publicar

- **Caminhos do Windows:** a tela Saúde do sistema mostra caminhos com o nome do usuário do computador. Troque esse trecho na página antes do print. No print atual, ele aparece como `pousada`.
- **Chave de recuperação:** nunca use `Mostrar chave de recuperação` durante os prints.
- **Sessão:** recarregar a página encerra a sessão, e a captura passa a mostrar a tela de login. Confira cada imagem.
- **Recorte:** telas curtas são recortadas para evitar área vazia.
