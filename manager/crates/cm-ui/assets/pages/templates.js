/* Templates page — phase 9 replaces this stub (templates list + form, extension library). */

I18N.add({
  en: { 'templates.soon': 'Reusable profile settings and a Firefox extension library will live here.' },
  vi: { 'templates.soon': 'Thiết lập profile dùng lại và thư viện extension Firefox sẽ nằm ở đây.' },
});

CM.pages.templates = {
  template: `<cm-page :title="t('nav.templates')">
    <cm-coming-soon icon="layers" :body="t('templates.soon')"/>
  </cm-page>`,
};
