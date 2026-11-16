/* Schedules page — phase 10 replaces this stub (scheduled warming launches, browser refresh, run log). */

I18N.add({
  en: { 'schedules.soon': 'Scheduled warming launches and browser refreshes will live here.' },
  vi: { 'schedules.soon': 'Lịch mở profile để "làm ấm" và cập nhật trình duyệt sẽ nằm ở đây.' },
});

CM.pages.schedules = {
  template: `<cm-page :title="t('nav.schedules')">
    <cm-coming-soon icon="calendar" :body="t('schedules.soon')"/>
  </cm-page>`,
};
